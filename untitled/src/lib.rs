use kinode_process_lib::{
    await_message, get_typed_state, get_blob, call_init, http::{
        bind_http_path, bind_ws_path, send_response, send_ws_push, serve_ui, HttpServerRequest,
        StatusCode, WsMessageType,
    }, our_capabilities, println, set_state, spawn, vfs::{
        create_drive, create_file, metadata, open_dir, open_file, remove_file, Directory, FileType
    }, Address, LazyLoadBlob, Message, OnExit, ProcessId, Request, Response
};
use serde_json;
use std::collections::{HashMap, HashSet};
use std::io::Read;

mod structs;
use structs::{SongDb, SongDbRequest, SongDbResponse, Song, Tag};

wit_bindgen::generate!({
    path: "target/wit",
    world: "process-v0",
});

call_init!(init);
fn init(our: Address) {
    println!("P2P Music Sharing: Initializing");

    let drive_path = create_drive(our.package_id(), "music_db", None).unwrap();
    let files_dir = open_dir(&drive_path, false, None).unwrap();
    let mut song_db = SongDb::load(&files_dir);
    let mut ws_channels: HashSet<u32> = HashSet::new();

    // Serve UI files
    serve_ui(&our, "ui", true, false, vec!["/"]).unwrap();

    // Bind HTTP paths
    bind_http_path("/get_songs_from_tag", true, false).unwrap();
    bind_http_path("/upload_song", true, false).unwrap();
    bind_http_path("/list_all_songs", true, false).unwrap();
    bind_http_path("/stream_audio", true, false).unwrap();

    // Bind WebSocket path
    bind_ws_path("/", true, false).unwrap();

    loop {
        match handle_message(&our, &mut song_db, &mut ws_channels) {
            Ok(()) => {}
            Err(e) => {
                println!("Error: {:?}", e);
                push_update_via_ws(&ws_channels, &format!("Error: {:?}", e));
            }
        }
    }
}

fn handle_message(
    our: &Address,
    song_db: &mut SongDb,
    ws_channels: &mut HashSet<u32>,
) -> anyhow::Result<()> {
    let message = await_message()?;

    match message {
        Message::Request { source, body, .. } => {
            if source.process.to_string() == "http_server:distro:sys" {
                handle_http_request(our, &source, &body, song_db, ws_channels)
            } else {
                handle_songdb_request(our, &source, &body, song_db, ws_channels)
            }
        }
        Message::Response { .. } => Ok(()),
    }
}

fn handle_songdb_request(
    our: &Address,
    source: &Address,
    body: &[u8],
    song_db: &mut SongDb,
    ws_channels: &mut HashSet<u32>,
) -> anyhow::Result<()> {
    let request = serde_json::from_slice::<SongDbRequest>(body)?;

    match request {
        SongDbRequest::GetSongsByTag(tag) => {
            let songs = song_db.get_songs_by_tag(&tag);
            Response::new()
                .body(serde_json::to_vec(&SongDbResponse::Songs(songs))?)
                .send()?;
        }
        SongDbRequest::UploadSong(upload_request) => {
            let blob = get_blob().ok_or_else(|| anyhow::anyhow!("No blob provided for song upload"))?;
            let song = Song {
                id: format!("{}.mp3", upload_request.name),
                name: upload_request.name,
                data: blob.bytes,
                tag: upload_request.tag,
            };
            match song_db.add_song(song) {
                Ok(_) => {
                    Response::new()
                        .body(serde_json::to_vec(&SongDbResponse::SongAdded)?)
                        .send()?;
                    push_update_via_ws(ws_channels, "Song uploaded successfully");
                }
                Err(e) => {
                    Response::new()
                        .body(serde_json::to_vec(&SongDbResponse::Error(format!("Failed to upload song: {}", e)))?)
                        .send()?;
                }
            }
        }
        SongDbRequest::GetAllTags => {
            let tags = song_db.get_all_tags();
            Response::new()
                .body(serde_json::to_vec(&SongDbResponse::Tags(tags.into_iter().cloned().collect()))?)
                .send()?;
        }
    }

    Ok(())
}

fn handle_http_request(
    our: &Address,
    source: &Address,
    body: &[u8],
    song_db: &mut SongDb,
    ws_channels: &mut HashSet<u32>,
) -> anyhow::Result<()> {
    let http_request = serde_json::from_slice::<HttpServerRequest>(body)?;

    match http_request {
        HttpServerRequest::Http(request) => {
            let method = request.method()?;
            let path = request.path()?;

            match (method.as_str(), path.as_str()) {
                ("GET", "/get_songs_from_tag") => {
                    let tag = request.query_params().get("tag").ok_or_else(|| anyhow::anyhow!("No tag provided"))?;
                    let songs = song_db.get_songs_by_tag(tag);
                    let response = serde_json::to_vec(&songs)?;
                    send_response(StatusCode::OK, Some(HashMap::from([("Content-Type".to_string(), "application/json".to_string())])), response);
                }

                ("GET", "/list_all_songs") => {
                    let all_songs: Vec<_> = song_db.songs.values().flatten().map(|song| {
                        serde_json::json!({
                            "id": song.id,
                            "name": song.name,
                            "tag": song.tag,
                        })
                    }).collect();
                    
                    let response = serde_json::to_vec(&all_songs)?;
                    send_response(StatusCode::OK, Some(HashMap::from([("Content-Type".to_string(), "application/json".to_string())])), response);
                }
                ("GET", "/stream_audio") => {
                    let song_id = request.query_params().get("id").ok_or_else(|| anyhow::anyhow!("No song ID provided"))?;
                    let file_path = format!("{}/{}", song_db.vfs_dir_path, song_id);
                    
                    match open_file(&file_path, false, None) {
                        Ok(file) => {
                            let data = file.read()?;
                            let mut headers = HashMap::new();
                            headers.insert("Content-Type".to_string(), "audio/mpeg".to_string());
                            headers.insert("Content-Length".to_string(), data.len().to_string());
                            send_response(StatusCode::OK, Some(headers), data);
                        },
                        Err(_) => {
                            send_response(StatusCode::NOT_FOUND, None, b"Audio file not found".to_vec());
                        }
                    }
                }
                ("POST", "/upload_song") => {

                    let blob = get_blob().ok_or_else(|| anyhow::anyhow!("No blob provided for song upload"))?;

                    println!("Received upload request with blob size: {}", blob.bytes.len());
    
                    // Create a longer-lived value for content_type
                    let content_type = request.headers()
                        .get("Content-Type")
                        .ok_or_else(|| anyhow::anyhow!("upload, Content-Type header not found"))?
                        .to_str()
                        .map_err(|_| anyhow::anyhow!("failed to convert Content-Type to string"))?
                        .to_owned(); // Convert to an owned String

                    println!("Content-Type: {}", content_type);
                
                    let boundary_parts: Vec<&str> = content_type.split("boundary=").collect();
                    let boundary = boundary_parts.get(1)
                        .ok_or_else(|| anyhow::anyhow!("upload fail, no boundary found in POST content type"))?;

                    println!("Boundary: {}", boundary);
                
                    let data = std::io::Cursor::new(blob.bytes);
                    let mut multipart = multipart::server::Multipart::with_body(data, *boundary);
                    
                    let mut name = String::new();
                    let mut tag_key = String::new();
                    let mut song_data = Vec::new();



                    while let Some(mut field) = multipart.read_entry().map_err(|e| anyhow::anyhow!("Error reading multipart entry: {:?}", e))? {
                        println!("Processing field: {}", field.headers.name);
                        match field.headers.name.as_ref() {
                            "name" => { field.data.read_to_string(&mut name)?; }
                            "tag" => { field.data.read_to_string(&mut tag_key)?; }
                            "file" => { 
                                if let Some(filename) = field.headers.filename.clone() {
                                    field.data.read_to_end(&mut song_data)?;
                                    println!("Uploaded file {} with size {}", filename, song_data.len());
                                } else {
                                    println!("File field found but no filename provided");
                                }
                            }
                            _ => { println!("Unexpected field: {}", field.headers.name); }
                        }
                    }

                    if name.is_empty() {
                        println!("Error: 'name' field is missing.");
                    }
                    if tag_key.is_empty() {
                        println!("Error: 'tag' field is missing.");
                    }
                    if song_data.is_empty() {
                        println!("Error: 'file' field is missing.");
                    }
                    
                    if name.is_empty() || tag_key.is_empty() || song_data.is_empty() {
                        send_response(StatusCode::BAD_REQUEST, None, b"Missing required fields for song upload".to_vec());
                        return Ok(());
                    }

                    println!("Creating song with name: {}, tag: {}, data size: {}", name, tag_key, song_data.len());

                    let song = Song {
                        id: format!("{}.mp3", name),
                        name,
                        data: song_data,
                        tag: Tag { key: tag_key.clone(), name: Some(tag_key) },
                    };

                    match song_db.add_song(song) {
                        Ok(_) => {
                            send_response(StatusCode::OK, None, b"Song uploaded successfully".to_vec());
                            push_update_via_ws(ws_channels, "Song uploaded successfully");
                        }
                        Err(e) => {
                            send_response(StatusCode::INTERNAL_SERVER_ERROR, None, format!("Failed to upload song: {}", e).into_bytes());
                        }
                    }
                }
                _ => {
                    send_response(StatusCode::NOT_FOUND, None, b"Not Found".to_vec());
                }
            }
        }
        HttpServerRequest::WebSocketOpen { channel_id, .. } => {
            ws_channels.insert(channel_id);
        }
        HttpServerRequest::WebSocketClose(channel_id) => {
            ws_channels.remove(&channel_id);
        }
        HttpServerRequest::WebSocketPush { .. } => {}
    }

    Ok(())
}

fn push_update_via_ws(ws_channels: &HashSet<u32>, update: &str) {
    for &channel_id in ws_channels {
        send_ws_push(
            channel_id,
            WsMessageType::Text,
            LazyLoadBlob {
                mime: Some("application/json".to_string()),
                bytes: serde_json::json!({
                    "type": "update",
                    "data": update
                })
                .to_string()
                .into_bytes(),
            },
        );
    }
}

