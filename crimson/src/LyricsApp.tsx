import { useState, useEffect, useRef } from 'react';
import { useLCU } from './contexts/LCUContext';
import { X, Maximize2, Minimize2, Play, Pause, SkipBack, SkipForward } from 'lucide-react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';

interface LyricLine {
  timeMs: number;
  text: string;
}

export default function LyricsApp() {
  const { spotifyState, spotifyCommand } = useLCU();
  const [lyrics, setLyrics] = useState<LyricLine[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isInstrumental, setIsInstrumental] = useState(false);
  const [noLyrics, setNoLyrics] = useState(false);
  const [localVideoPath, setLocalVideoPath] = useState<string | null>(null);
  const [isDownloading, setIsDownloading] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isHovered, setIsHovered] = useState(false);

  const videoRef = useRef<HTMLVideoElement>(null);
  
  // Sync state
  const [currentTimeMs, setCurrentTimeMs] = useState(0);
  const syncRef = useRef({ progressMs: 0, localTimeAtSync: 0, isPlaying: false });

  // Refs for scrolling
  const containerRef = useRef<HTMLDivElement>(null);
  const activeLineRef = useRef<HTMLDivElement>(null);

  // Parse LRC format
  const parseLrc = (lrc: string): LyricLine[] => {
    const lines = lrc.split('\n');
    const parsed: LyricLine[] = [];
    const timeRegex = /\[(\d{2}):(\d{2})\.(\d{2,3})\]/;
    
    for (const line of lines) {
      const match = line.match(timeRegex);
      if (match) {
        const mins = parseInt(match[1], 10);
        const secs = parseInt(match[2], 10);
        const ms = parseInt(match[3], 10) * (match[3].length === 2 ? 10 : 1);
        const text = line.replace(timeRegex, '').trim();
        
        parsed.push({
          timeMs: (mins * 60 + secs) * 1000 + ms,
          text: text || ' ' // Keep empty lines for pacing
        });
      }
    }
    return parsed.sort((a, b) => a.timeMs - b.timeMs);
  };

  // Fallback to YouTube via Rust backend
  const searchYouTubeVideo = async (track: string, artist: string) => {
    setIsDownloading(true);
    try {
      const query = `${artist} ${track} official music video`;
      const videoId: string = await invoke('youtube_search', { query });
      if (videoId) {
        const localPath: string = await invoke('download_music_video', { 
            videoId,
            artist,
            track
        });
        if (localPath) {
            setLocalVideoPath(localPath);
            setIsDownloading(false);
            return;
        }
      }
    } catch (e) {
      console.error("YouTube search/download failed via backend", e);
    }
    setIsDownloading(false);
    setNoLyrics(true);
  };

  // Fetch lyrics from LRCLIB
  useEffect(() => {
    if (!spotifyState?.track_id || !spotifyState?.track_name || !spotifyState?.artist_name) return;

    const fetchLyrics = async () => {
      setIsLoading(true);
      setNoLyrics(false);
      setIsInstrumental(false);
      setLyrics([]);
      setLocalVideoPath(null);
      setIsDownloading(false);
      
      try {
        const url = `https://lrclib.net/api/get?track_name=${encodeURIComponent(spotifyState.track_name)}&artist_name=${encodeURIComponent(spotifyState.artist_name)}&duration=${Math.round(spotifyState.duration_ms / 1000)}`;
        const res = await fetch(url);
        
        if (res.ok) {
          const data = await res.json();
          if (data.instrumental) {
            setIsInstrumental(true);
          } else if (data.syncedLyrics) {
            setLyrics(parseLrc(data.syncedLyrics));
          } else {
            searchYouTubeVideo(spotifyState.track_name, spotifyState.artist_name);
          }
        } else {
          searchYouTubeVideo(spotifyState.track_name, spotifyState.artist_name);
        }
      } catch (e) {
        console.error("Failed to fetch lyrics:", e);
        searchYouTubeVideo(spotifyState.track_name, spotifyState.artist_name);
      } finally {
        setIsLoading(false);
      }
    };

    fetchLyrics();
  }, [spotifyState?.track_id]);

  // Keep progress in sync
  useEffect(() => {
    if (spotifyState) {
      syncRef.current = {
        progressMs: spotifyState.progress_ms,
        localTimeAtSync: performance.now(),
        isPlaying: spotifyState.is_playing
      };
    }
  }, [spotifyState?.progress_ms, spotifyState?.is_playing]);

  // Interpolation loop
  useEffect(() => {
    let animationFrameId: number;

    const tick = () => {
      const now = performance.now();
      const sync = syncRef.current;
      
      if (sync.isPlaying) {
        const diff = now - sync.localTimeAtSync;
        setCurrentTimeMs(sync.progressMs + diff);
      } else {
        setCurrentTimeMs(sync.progressMs);
      }
      
      animationFrameId = requestAnimationFrame(tick);
    };
    
    animationFrameId = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(animationFrameId);
  }, []);

  // Find active line
  const activeIndex = lyrics.findIndex((line, index) => {
    const nextLine = lyrics[index + 1];
    if (!nextLine) return currentTimeMs >= line.timeMs;
    return currentTimeMs >= line.timeMs && currentTimeMs < nextLine.timeMs;
  });

  // Smooth scrolling for lyrics
  useEffect(() => {
    if (activeLineRef.current) {
      activeLineRef.current.scrollIntoView({
        behavior: 'smooth',
        block: 'center'
      });
    }
  }, [activeIndex]);

  // Sync Video
  useEffect(() => {
    if (videoRef.current && localVideoPath) {
      const video = videoRef.current;
      const progressSec = (spotifyState?.progress_ms || 0) / 1000;
      
      // If diff > 1 sec, seek
      if (Math.abs(video.currentTime - progressSec) > 1.0) {
        video.currentTime = progressSec;
      }
      
      if (spotifyState?.is_playing) {
        if (video.paused) {
          video.play().catch(e => console.error(e));
        }
      } else {
        if (!video.paused) {
          video.pause();
        }
      }
    }
  }, [spotifyState?.progress_ms, spotifyState?.is_playing, localVideoPath]);

  const toggleFullscreen = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const win = getCurrentWindow();
      const isFs = await win.isFullscreen();
      if (isFs) {
        await win.setFullscreen(false);
        setIsFullscreen(false);
      } else {
        await win.setFullscreen(true);
        setIsFullscreen(true);
      }
    } catch(e) { console.error(e) }
  };

  const closeWindow = async () => {
    try {
      const appWindow = getCurrentWebviewWindow();
      await appWindow.close();
    } catch (e) {
      console.error("Failed to close window", e);
    }
  };



  return (
    <div 
      className="relative w-screen h-screen overflow-hidden bg-black selection:bg-white/20 font-sans text-white group"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      data-tauri-drag-region
    >
      {/* Background Layer: Blurred Album Art */}
      <div 
        className="absolute inset-0 bg-cover bg-center bg-no-repeat transition-all duration-1000 scale-110"
        style={{ 
          backgroundImage: spotifyState?.album_art ? `url(${spotifyState.album_art})` : 'none',
          filter: 'blur(60px) brightness(0.3)' 
        }}
        data-tauri-drag-region
      />
      
      {/* Top Bar (Draggable) */}
      <div data-tauri-drag-region className="absolute top-0 left-0 right-0 h-16 flex items-center justify-between px-6 z-50 bg-gradient-to-b from-black/50 to-transparent">
        <div data-tauri-drag-region className="flex items-center gap-4">
          {spotifyState?.album_art && (
            <img src={spotifyState.album_art} alt="Album" className="w-10 h-10 rounded-lg shadow-lg" />
          )}
          <div className="pointer-events-none">
            <h2 className="text-white font-bold text-sm leading-tight drop-shadow-md">
              {spotifyState?.track_name || 'Aucune musique'}
            </h2>
            <p className="text-white/60 text-xs drop-shadow-md">
              {spotifyState?.artist_name || 'Spotify'}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={toggleFullscreen} className="p-2 hover:bg-white/10 rounded-full transition-colors text-white/50 hover:text-white">
            {isFullscreen ? <Minimize2 className="w-5 h-5" /> : <Maximize2 className="w-5 h-5" />}
          </button>
          <button onClick={closeWindow} className="p-2 hover:bg-red-500/80 rounded-full transition-colors text-white/50 hover:text-white">
            <X className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Lyrics Container */}
      <div 
        ref={containerRef}
        className="relative z-10 w-full h-full flex flex-col items-center pt-[40vh] pb-[60vh] overflow-y-auto scrollbar-hide scroll-smooth px-8 lg:px-24"
        style={{ msOverflowStyle: 'none', scrollbarWidth: 'none' }}
      >
        {isLoading ? (
          <div className="text-2xl font-bold text-white/50 animate-pulse mt-20">Chargement des paroles...</div>
        ) : isInstrumental ? (
          <div className="text-3xl font-black text-white/80 mt-20 tracking-widest uppercase">Instrumental</div>
        ) : lyrics.length > 0 ? (
          lyrics.map((line, idx) => {
            const isActive = idx === activeIndex;
            const isPassed = idx < activeIndex;
            
            return (
              <div 
                key={idx}
                ref={isActive ? activeLineRef : null}
                className={`w-full max-w-4xl text-center transition-all duration-500 transform py-4 ${
                  isActive 
                    ? 'text-4xl md:text-5xl lg:text-6xl font-black text-white drop-shadow-[0_0_15px_rgba(255,255,255,0.3)] scale-100 opacity-100' 
                    : isPassed
                      ? 'text-3xl md:text-4xl lg:text-5xl font-bold text-white/40 scale-95 opacity-50'
                      : 'text-3xl md:text-4xl lg:text-5xl font-bold text-white/30 scale-95 opacity-40 blur-[1px]'
                }`}
                style={{
                  transitionTimingFunction: 'cubic-bezier(0.2, 0.8, 0.2, 1)'
                }}
              >
                {line.text === ' ' ? '...' : line.text}
              </div>
            );
          })
        ) : localVideoPath ? (
          <div className="absolute inset-0 w-full h-full pointer-events-none opacity-40">
            <video 
              ref={videoRef}
              src={convertFileSrc(localVideoPath)}
              className="w-full h-full object-cover"
              muted
              loop
              playsInline
            />
          </div>
        ) : isDownloading ? (
          <div className="flex flex-col items-center justify-center h-full w-full opacity-60 mt-20">
             <div className="text-xl font-bold text-white mb-2 animate-pulse">Téléchargement du clip en haute qualité...</div>
             <div className="text-sm text-white/50">Cela peut prendre quelques secondes la première fois.</div>
          </div>
        ) : noLyrics ? (
          <div className="text-2xl font-bold text-white/50 mt-20">Paroles non disponibles pour ce morceau.</div>
        ) : null}
      </div>

      {/* Media Controls Overlay */}
      <div className={`absolute bottom-10 left-1/2 -translate-x-1/2 flex items-center gap-6 px-8 py-4 bg-black/40 backdrop-blur-xl border border-white/10 rounded-3xl shadow-2xl transition-all duration-500 z-50 ${isHovered ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-8 pointer-events-none'}`}>
        <button 
          onClick={closeWindow}
          className="p-3 rounded-full hover:bg-red-500/20 hover:text-red-400 transition-colors text-white/50"
        >
          <X className="w-5 h-5" />
        </button>

        <div className="w-px h-8 bg-white/10 mx-2" />

        <button 
          onClick={() => spotifyCommand('previous')}
          className="p-3 rounded-full hover:bg-white/10 transition-colors text-white"
        >
          <SkipBack className="w-6 h-6" />
        </button>
        <button 
          onClick={() => spotifyCommand('play_pause')}
          className="p-4 rounded-full bg-white text-black hover:scale-105 active:scale-95 transition-all shadow-[0_0_20px_rgba(255,255,255,0.3)]"
        >
          {spotifyState?.is_playing ? <Pause className="w-8 h-8 fill-current" /> : <Play className="w-8 h-8 fill-current ml-1" />}
        </button>
        <button 
          onClick={() => spotifyCommand('next')}
          className="p-3 rounded-full hover:bg-white/10 transition-colors text-white"
        >
          <SkipForward className="w-6 h-6" />
        </button>

        <div className="w-px h-8 bg-white/10 mx-2" />

        <button 
          onClick={toggleFullscreen}
          className="p-3 rounded-full hover:bg-white/10 transition-colors text-white/50 hover:text-white"
        >
          {isFullscreen ? <Minimize2 className="w-5 h-5" /> : <Maximize2 className="w-5 h-5" />}
        </button>
      </div>
    </div>
  );
}
