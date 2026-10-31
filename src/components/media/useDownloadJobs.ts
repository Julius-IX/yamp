import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { COMMANDS } from "../../lib/commands";
import type { DownloadJob } from "../../lib/types";

/** Emitted by the backend with a snapshot of a job whenever it changes. */
const DOWNLOAD_EVENT = "yt-api:download";

const active = (j: DownloadJob) => j.state === "queued" || j.state === "running";

/** Mirrors the backend order. A job requested again is a new entry at the end. */
function upsert(prev: DownloadJob[], job: DownloadJob): DownloadJob[] {
  if (job.state === "queued") return [...prev.filter((j) => j.id !== job.id), job];
  return prev.map((j) => (j.id === job.id ? job : j));
}

/** Download queue state kept in sync with the backend. */
export function useDownloadJobs() {
  const [jobs, setJobs] = useState<DownloadJob[]>([]);

  useEffect(() => {
    let alive = true;
    // subscribing first means a change that lands while the list loads is not lost
    const off = listen<DownloadJob>(DOWNLOAD_EVENT, (e) => setJobs((prev) => upsert(prev, e.payload)));
    COMMANDS.queue.list().then((known) => {
      // events are newer than the list, so they win for jobs present in both
      if (alive) setJobs((prev) => [...known.filter((k) => !prev.some((j) => j.id === k.id)), ...prev]);
    });
    return () => {
      alive = false;
      off.then((f) => f());
    };
  }, []);

  const clearFinished = useCallback(async () => {
    await COMMANDS.queue.clearFinished();
    setJobs((prev) => prev.filter(active));
  }, []);

  return { jobs, clearFinished };
}
