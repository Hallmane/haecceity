import React, { useState, useRef, useEffect } from 'react';

interface Song {
  id: string;
  name: string | null;
  tag: {
    key: string;
    name: string | null;
  };
}

const SongSearch: React.FC = () => {
  const [tagKey, setTagKey] = useState('');
  const [songs, setSongs] = useState<Song[]>([]);
  const [allSongs, setAllSongs] = useState<Song[]>([]);

  const [uploadFile, setUploadFile] = useState<File | null>(null);
  const [uploadTag, setUploadTag] = useState('');

  const audioRef = useRef<HTMLAudioElement>(null);

  const fetchAllSongs = async () => {
    try {
      const response = await fetch(`${import.meta.env.BASE_URL}/list_all_songs`);
      if (!response.ok) {
        throw new Error('Failed to fetch all songs')
      }
      const data = await response.json();
      setAllSongs(data);
    } catch (err) {
      console.error('Error fetching all songs:', err);
    }
  };


  useEffect(() => {
    // Function to load initial songs
    const loadInitialSongs = async () => {
      try {
        const response = await fetch(`${import.meta.env.BASE_URL}/get_songs_from_tag?tag=defaultKey`);
        if (!response.ok) {
          throw new Error('Failed to fetch songs');
        }
        const data = await response.json();
        setSongs(data || []);  // Adjust based on the actual data structure
      } catch (err) {
        console.error('Error fetching songs:', err);
      }
    };

    // Call the function on component mount
    loadInitialSongs();
  }, []);  // Empty dependency array means this effect runs only once on mount

  useEffect(() => {
    // Automatically play the first song when new songs are loaded
    if (songs.length > 0 && audioRef.current) {
      playSong(songs[0]);
    }
  }, [songs]);  // This effect runs whenever the `songs` state changes

  const searchSongs = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      const response = await fetch(`${import.meta.env.BASE_URL}/get_songs_from_tag?tag=${tagKey}`);
      if (!response.ok) {
        throw new Error('Failed to fetch songs');
      }
      const data = await response.json();
      console.log("logging data:", data);
      setSongs(data || []);
    } catch (err) {
      console.error('Error fetching songs:', err);
    }
  };

  const playSong = (song: Song) => {
    if (audioRef.current) {
      audioRef.current.src = `${import.meta.env.BASE_URL}/stream_audio?id=${encodeURIComponent(song.id)}`;
      audioRef.current.play();
    }
  };

  // song upload stuff
  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files) {
      setUploadFile(e.target.files[0]);
    }
  };

  const uploadSong = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!uploadFile || !uploadTag) {
      alert('Please select a file and enter a tag');
      return;
    }
    const formData = new FormData();
    formData.append('file', uploadFile);
    formData.append('tag', uploadTag);
    formData.append('name', "fake_name");

    try {
      const response = await fetch(`${import.meta.env.BASE_URL}/upload_song`, {
        method: 'POST',
        body: formData,
      });

      if (!response.ok) {
        throw new Error('Failed to upload song');
      }

      alert('Song uploaded successfully');
      setUploadFile(null);
      setUploadTag('');
      // Optionally, refresh the song list here
    } catch (err) {
      console.error('Error uploading song:', err);
      alert('Failed to upload song');
    }
  };
  // ^ song upload stuff ^
  return (
    <div>
      <h2>Song Search</h2>
      <form onSubmit={searchSongs}>
        <input 
          type="text" 
          value={tagKey} 
          onChange={(e) => setTagKey(e.target.value)}
          placeholder="Enter tag key"
        />
        <button type="submit">Search</button>
      </form>
      <div>
        <h3>Results:</h3>
        {songs.length === 0 ? (
          <p>No songs found for this tag.</p>
        ) : (
          <ul>
            {songs.map((song) => (
              <li key={song.id}>
                {song.name} - Tag: {song.tag.name || song.tag.key}
                <button onClick={() => playSong(song)}>Play</button>
              </li>
            ))}
          </ul>
        )}
      </div>
      <audio ref={audioRef} controls />
      <h2>Upload Song</h2>
      <form onSubmit={uploadSong}>
        <input 
          type="file" 
          accept=".mp3"
          onChange={handleFileChange}
        />
        <input 
          type="text" 
          value={uploadTag} 
          onChange={(e) => setUploadTag(e.target.value)}
          placeholder="Enter tag for the song"
        />
        <button type="submit">Upload</button>
      </form>
      <h2>All Songs</h2>
      <button onClick={fetchAllSongs}>Fetch All Songs</button>
      <ul>
        {allSongs.map((song) => (
          <li key={song.id}>
              {song.name} - Tag: {song.tag.name || song.tag.key}
          </li>
        ))}
      </ul>
    </div>
  );
};

export default SongSearch;