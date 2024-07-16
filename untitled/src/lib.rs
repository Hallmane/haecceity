use std::fs;
use std::collections::HashSet;
use kinode_process_lib::{await_message, call_init, Address, Message, Request, Response, get_blob, println, clear_state,
                        http::{serve_ui, bind_http_path, bind_ws_path},
                        vfs::{create_drive, open_dir}
                    };

mod structs;
use structs::{SongDb, SongDbRequest, SongDbResponse, MP3File, Song, Tag};



wit_bindgen::generate!({
    path: "target/wit",
    world: "process-v0",
});

call_init!(init);
fn init(our: Address) {
    println!("begin");

    //clear_state();

    //let drive_path = create_drive(our.package_id(), "record_shop", None).unwrap();
    let drive_path = match create_drive(our.package_id(), "record_shop", None) {
        Ok(path) => path,
        Err(e) => {
            println!("Error creating drive: {:?}. Attempting to open existing drive.", e);
            format!("{}/record_shop", our.package_id())
        }
    };

    //let vfs_dir = open_dir(&drive_path, true, Some(5)).unwrap();
    let vfs_dir = match open_dir(&drive_path, false, None) {
        Ok(dir) => dir,
        Err(e) => {
            println!("Error opening directory: \n{:?}. \nExiting.", e);
            return;
        }
    };


    let mut song_db = SongDb::new(&vfs_dir);

    match add_local_files_to_vfs(&mut song_db, "music_files") {
        Ok(_) => print!("local files added to vfs"),
        Err(_) => print!("error adding the local files to the vfs called: {:?}", song_db),
    }

    //let file = open_file(&file_path, true);
    //let file_path = format!("{}/hello.txt", &drive_path);


    //let mut message_archive: MessageArchive = HashMap::new();
    let mut channel_id = 0;

    // Bind UI files to routes; index.html is bound to "/"
    serve_ui(&our, "ui", true, true, vec!["/"]).unwrap();

    // front end api end points
    bind_http_path("/get_songs_from_key", true, false).unwrap(); // front end hits this api endpoint with a query (tag key) and receives the matched songs.

    // Bind WebSocket path 
    bind_ws_path("/", true, false).unwrap();

    loop {
        match handle_message(&our, &mut channel_id, &mut song_db) {
            Ok(()) => {}
            Err(e) => {
                println!("error: {:?}", e);
            }
        };
    }
}

fn handle_message(our: &Address, channel_id: &mut u64, song_db: &mut SongDb) -> anyhow::Result<()> {
    //let message = await_message(channel_id).unwrap();
    let message = await_message()?;

    if message.source().node != our.node {
        return Ok(()); // ignore messages from other nodes for now
    }

    match message {
        Message::Request { ref body, ..} => { handle_request(song_db, body) }
        Message::Response { .. } => Ok(()),
        _ => { return Err(anyhow::anyhow!("unhandled message type")) }
    }
}

fn handle_request(song_db: &mut SongDb, body: &[u8]) -> anyhow::Result<()> {
    let request : SongDbRequest = serde_json::from_slice(body)?;

    let response = match request {
        SongDbRequest::GetSongsByTag(tag) => {
            match song_db.get_songs_by_tag(&tag) {
                Some(songs) => {
                    let songs_without_data: Vec<Song> = songs.iter().map(|s| Song {
                        id: s.id.clone(),
                        name: s.name.clone(),
                        data: vec![],
                        tag: s.tag.clone(),
                    }).collect();
                    SongDbResponse::Songs(songs_without_data)
                },
                None => SongDbResponse::Songs(vec![]),
            }
        }

        // this should be matched when the front end sends a play request.
        SongDbRequest::GetSong(song_id) => {
            match song_db.songs.values().flat_map(|songs| songs.iter()).find(|s| s.id == song_id) {
                Some(song) => {
                    match song_db.get_song_data(&song_id) {
                        Ok(song_data) => SongDbResponse::Song(Song {
                            id: song.id.clone(),
                            name: song.name.clone(),
                            data: song_data,
                            tag: song.tag.clone(),
                        }),
                        Err(_) => SongDbResponse::Error("Failed to retrieve song data".to_string()),
                    }
                },
                None => SongDbResponse::Error("Song not found".to_string()),
            }
        }

        SongDbRequest::GetAllTags => {
            let tags = song_db.get_all_tags();
            SongDbResponse::Tags(tags.into_iter().cloned().collect())
        }

        SongDbRequest::AddSong(song) => {
            match song_db.add_song(song) {
                Ok(_) => SongDbResponse::SongAdded,
                Err(_) => SongDbResponse::Error("Failed to add song".to_string()),
            }
        }
    };

    let response_body = serde_json::to_vec(&response)?;
    Response::new().body(response_body).send()?;
    Ok(())

}


fn add_local_files_to_vfs(song_db: &mut SongDb, local_dir: &str) -> anyhow::Result<()> {
    for entry in fs::read_dir(local_dir)? {
        print!("trying to add {:?}", &entry);
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("mp3") {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            let file_content = fs::read(&path)?;
            let tag = Tag {
                key: "uuid-1".to_string(),
                name: Some("eine_kleine_klitz_musik".to_string()),
            };
            let song = Song {
                id: file_name.to_string(),
                name: file_name.to_string(),
                data: file_content,
                tag: tag,
            };
            song_db.add_song(song)?;
        }
        print!("{:?} added", &entry);
    }
    Ok(())
}