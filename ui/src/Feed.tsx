import React, { useState, useEffect, useRef } from 'react';

const BASE_URL = import.meta.env.BASE_URL;

interface MP3File {
    name: string;
    url: string;
}

const Feed: React.FC = () => {
    //const [mp3Files, setMp3Files] = useState<MP3File[]>([]);
    const [currentFile, setCurrentFile] = useState<string | undefined>(undefined);
    const fileInputRef = useRef<HTMLInputElement>(null);

    const handleFiles = (event: React.ChangeEvent<HTMLInputElement>) => {
        const files = event.target.files;
        if (files && files[0]) {
            const fileURL = URL.createObjectURL(files[0]);
            setCurrentFile(fileURL);
        }
    };

    //useEffect(() => {
    //     Fetch the list of MP3 files from the network through kinode
    //    const fetchMP3Files = async () => {
    //        try {
    //            const response = await fetch(`${BASE_URL}/mp3files`); // Adjust the URL based on your server setup
    //            const files: MP3File[] = await response.json();
    //            setMp3Files(files);
    //        } catch (error) {
    //            console.error('Failed to fetch MP3 files:', error);
    //        }
    //    };

    //    fetchMP3Files();
    //}, []);

    //const handleFileSelect = (url: string) => {
    //    setCurrentFile(url);
    //};

    return (
        <div>
        <h3>Local Music Player</h3>
        <input
            type="file"
            ref={fileInputRef}
            onChange={handleFiles}
            accept="audio/mpeg"
            style={{ display: 'block', margin: '20px 0' }}
        />
        {currentFile && (
            <audio controls src={currentFile} autoPlay>
                Your browser does not support the audio tag.
            </audio>
        )}
    </div>
    );
};

export default Feed;