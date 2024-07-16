import React, { useState, useRef, useEffect } from 'react';

interface Song {
  id: string;
  playable_media: {
    path: string;
    name: string | null;
  };
  tag: {
    key: string;
    name: string | null;
  };
}

const SongSearch: React.FC = () => {
  const [tagKey, setTagKey] = useState('');
  const [songs, setSongs] = useState<Song[]>([]);
  const audioRef = useRef<HTMLAudioElement>(null);

  useEffect(() => {
    // Function to load initial songs
    const loadInitialSongs = async () => {
      try {
        const response = await fetch(`${import.meta.env.BASE_URL}/get_songs_from_key?tag_key=defaultKey`);
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
      const response = await fetch(`${import.meta.env.BASE_URL}/get_songs_from_key?tag_key=${tagKey}`);
      if (!response.ok) {
        throw new Error('Failed to fetch songs');
      }
      const data = await response.json();
      setSongs(data || []);
    } catch (err) {
      console.error('Error fetching songs:', err);
    }
  };

  const playSong = (song: Song) => {
    if (audioRef.current) {
      audioRef.current.src = `${import.meta.env.BASE_URL}/get_audio?path=${encodeURIComponent(song.playable_media.path)}`;
      audioRef.current.play();
    }
  };

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
                {song.playable_media.name} - Tag: {song.tag.name || song.tag.key}
                <button onClick={() => playSong(song)}>Play</button>
              </li>
            ))}
          </ul>
        )}
      </div>
      <audio ref={audioRef} controls />
    </div>
  );
};

export default SongSearch;