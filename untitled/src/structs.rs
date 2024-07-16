use std::collections::HashMap;
use kinode_process_lib::{set_state, get_state, vfs};
use kinode_process_lib::http::{HttpServerRequest, IncomingHttpRequest, WsMessageType };
use kinode_process_lib::vfs::Directory;
use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub enum IncomingMessage {
    Http(IncomingHttpRequest),
    WebSocketOpen { path: String, channel_id: u32 },
    WebSocketClose(u32),
    WebSocketPush { channel_id: u32, message_type: WsMessageType },
    SongDb(SongDbRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SongDbRequest{
    GetSongsByTag(String),
    GetSong(String),
    GetAllTags,
    AddSong(Song),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SongDbResponse {
    Songs(Vec<Song>),
    Song(Song),
    Tags(Vec<String>),
    SongAdded,
    //SongRemoved(bool),
    Error(String),
} 

#[derive(Debug, Serialize, Deserialize)]
pub struct SongDb {
    pub vfs_dir_path: String,
    pub songs: HashMap<String, Vec<Song>>,
}
impl SongDb {
    pub fn new(vfs_dir: &Directory) -> Self {
        Self {
            vfs_dir_path: vfs_dir.path.to_string(),
            songs: HashMap::new(),
        }
    }

    pub fn load(vfs_dir: &Directory) -> Self {
        match get_state() {
            Some(state_bytes) => {
                let mut db: SongDb = bincode::deserialize(&state_bytes)
                    .unwrap_or_else(|_| Self::new(vfs_dir));
                db.vfs_dir_path = vfs_dir.path.to_string();
                db
            },
            None => Self::new(vfs_dir)
        }
    }

    pub fn save(&self) {
        let state_bytes = bincode::serialize(self).expect("Failed to serialize state");
        set_state(&state_bytes);
    }

    pub fn add_song(&mut self, mut song: Song) -> anyhow::Result<()> {
        let file_path = format!("{}/{}", self.vfs_dir_path, song.id);
        let mut file = vfs::create_file(&file_path, Some(5))?;
        file.write_all(&song.data)?;

        // Clear the data after writing to file to save memory
        song.data.clear();

        self.songs.entry(song.tag.key.clone())
            .or_insert_with(Vec::new)
            .push(song);

        self.save();
        Ok(())
    }

    pub fn get_songs_by_tag(&self, tag: &str) -> Option<&Vec<Song>> {
        self.songs.get(tag)
    }

    // so will this be the thing that the backend hits when the front end asks for playback of a single file?
    pub fn get_song_data(&self, song_id: &str) -> anyhow::Result<Vec<u8>> {
        let file_path = format!("{}/{}", self.vfs_dir_path, song_id);
        let file = vfs::open_file(&file_path, true, Some(5))?;
        let data = Vec::new();
        file.read_to_end()?;
        Ok(data)
    }

    pub fn get_all_tags(&self) -> Vec<&String> {
        self.songs.keys().collect()
    }

    pub fn remove_songs_by_tag(&mut self, tag_key: &str) -> bool {
        self.songs.remove(tag_key).is_some()
    }

}

#[derive(Eq, Hash, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub key: String, 
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Song {
    pub id: String, 
    pub name: String,
    pub data: Vec<u8>,
    pub tag: Tag, 
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PlayableMedia {
    MP3File(MP3File),
    //more
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MP3File {
    pub name: String, 
    pub data: Vec<u8>,
    //pub path: String,
}

//#[derive(Debug, Serialize, Deserialize)]
//pub struct SongDb { 
//    songs: HashMap<String, Vec<Song>>, // tag_key: songs
//}
//impl SongDb {
//    pub fn new() -> Self {
//        Self {
//            songs: HashMap::new(),
//        }
//    }
//
//    pub fn add_song(&mut self, song: Song) {
//        let tag_key = song.tag.key.clone();
//        self.songs.entry(tag_key)
//            .or_insert_with(Vec::new)
//            .push(song);
//    }
//
//    pub fn get_songs_by_tag(&self, tag_key: &str) -> Option<&Vec<Song>> {
//        self.songs.get(tag_key)
//    }
//
//    pub fn remove_songs_by_tag(&mut self, tag_key: &str) -> bool {
//        self.songs.remove(tag_key).is_some()
//    }
//
//    pub fn get_all_tags(&self) -> Vec<&String> {
//        self.songs.keys().collect()
//    }
//}