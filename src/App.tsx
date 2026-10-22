import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Region = { x: number; y: number; w: number; h: number };
type Progress = { phase: string; frame?: number };

export default function App() {
  const [region, setRegion] = useState<Region | null>(null);
  const [running, setRunning] = useState(false);
  const [countdown, setCountdown] = useState<number | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [savedFolder, setSavedFolder] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  
  const [scrollDelay, setScrollDelay] = useState(500);
  const [scrollSteps, setScrollSteps] = useState(10); // Default to 10 down-arrows

  useEffect(() => {
    const un = listen<Progress>("progress", (e) => setProgress(e.payload));
    return () => { un.then((f) => f()); };
  }, []);

  async function selectRegion() {
    setError(null);
    try {
      const r = await invoke<Region | null>("pick_region");
      if (r) {
        setRegion(r);
        setSavedFolder(null);
        setProgress(null);
      }
    } catch (e) { setError(String(e)); }
  }

  async function doCapture() {
    if (!region) return;
    setRunning(true);
    setError(null);
    setSavedFolder(null);
    setProgress({ phase: "capture" });
    try {
      const folder = await invoke<string | null>("start_capture", {
        region,
        scrollKey: "Down", // Use Down arrow instead of Page_Down
        scrollSteps,
        scrollDelayMs: scrollDelay,
      });
      if (folder) setSavedFolder(folder);
      else setProgress(null);
    } catch (e) { setError(String(e)); } 
    finally { setRunning(false); }
  }

  async function startWithCountdown() {
    if (!region) return;
    for (let i = 3; i >= 1; i--) {
      setCountdown(i);
      await new Promise((r) => setTimeout(r, 1000));
    }
    setCountdown(null);
    doCapture();
  }

  async function snapManual() {
    if (!region) return;
    try {
      await invoke("capture_single_frame", { region });
    } catch (e) { setError(String(e)); }
  }

  async function cancel() {
    try { await invoke("cancel_capture"); } catch (e) {}
  }

  async function openFolder() {
    if (!savedFolder) return;
    try { await invoke("open_folder", { path: savedFolder }); } catch (e) { setError(String(e)); }
  }

  return (
    <div className="p-5 space-y-4 text-sm">
      <h1 className="text-lg font-semibold">Scrollshot</h1>
      
      <button className="btn" onClick={selectRegion} disabled={running || countdown !== null}>
        {region ? "Re-select region" : "Select region"}
      </button>

      {region && <p className="text-xs text-gray-400">{region.w}×{region.h} at ({region.x}, {region.y})</p>}

      <div className="space-y-3 border-t border-[#2a3136] pt-3">
        <label className="block text-xs">
          Auto-Scroll Steps (Down Arrow presses)
          <input
            type="number"
            min={1}
            max={50}
            value={scrollSteps}
            onChange={(e) => setScrollSteps(Number(e.target.value))}
            className="input"
          />
          <span className="block text-[10px] text-gray-600 mt-1">
            Lower = more overlap. Higher = less overlap. (10 is roughly half a page).
          </span>
        </label>

        <label className="block text-xs">
          Scroll delay (ms)
          <input
            type="number"
            min={100}
            max={3000}
            step={50}
            value={scrollDelay}
            onChange={(e) => setScrollDelay(Number(e.target.value))}
            className="input"
          />
        </label>
      </div>

      <div className="border-t border-[#2a3136] pt-3 space-y-2">
        {countdown !== null ? (
          <p className="text-2xl font-bold text-blue-400">Click chat — starting in {countdown}…</p>
        ) : running ? (
          <button className="btn text-red-400" onClick={cancel}>Cancel Auto-Capture</button>
        ) : (
          <>
            <button className="btn bg-blue-600 text-white" onClick={startWithCountdown} disabled={!region}>
              Auto-Capture (Scroll & Snap)
            </button>
            <button className="btn" onClick={snapManual} disabled={!region}>
              Snap Frame Manually (No Scroll)
            </button>
          </>
        )}
      </div>

      {progress && <p className="text-xs text-blue-300">{progress.phase} {progress.frame ? `— frame ${progress.frame}` : ""}</p>}
      {savedFolder && (
        <div className="space-y-1">
          <p className="text-xs text-green-400 break-all">Saved to: {savedFolder}</p>
          <button className="btn" onClick={openFolder}>Open Folder</button>
        </div>
      )}
      {error && <p className="text-xs text-red-400 break-all">{error}</p>}
    </div>
  );
}