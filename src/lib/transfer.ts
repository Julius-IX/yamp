import { listen } from "@tauri-apps/api/event";
import { COMMANDS } from "./commands";
import { refreshPlaylists } from "./library";
import { refreshLikes } from "./likes";
import { logger } from "./log";
import type { FailedEntry } from "./types";

const log = logger("transfer");

/** Backend event while an import looks songs up, see `api/transfer.rs`. */
const PROGRESS_EVENT = "transfer:progress";

export type ImportProgress = {
  /** Which of the chosen files is being read, from 1. */
  file: number;
  files: number;
  done: number;
  total: number;
};

/** A failure with the file it came from, since row numbers of different files overlap. */
export type FileFailure = FailedEntry & { file: string };

export type ImportSummary = {
  sources: string[];
  total: number;
  liked: number;
  playlists: number;
  albums: number;
  failures: FileFailure[];
  /** All failures as one file that can be imported again. */
  failedCsv: string;
  /** Files that could not be read at all, with the reason. */
  unreadable: string[];
};

/** Every file has the same header, so only the first one keeps it. */
const withoutHeader = (csv: string) => csv.slice(csv.indexOf("\n") + 1);

/**
 * Imports the files one after the other. The text is read here and handed to the backend, since
 * a phone gives a picked file as a content URI that the backend cannot open.
 */
export async function importFiles(files: File[], onProgress: (p: ImportProgress) => void): Promise<ImportSummary> {
  const summary: ImportSummary = {
    sources: [],
    total: 0,
    liked: 0,
    playlists: 0,
    albums: 0,
    failures: [],
    failedCsv: "",
    unreadable: [],
  };
  let current = 1;
  const off = listen<{ done: number; total: number }>(PROGRESS_EVENT, (e) =>
    onProgress({ file: current, files: files.length, ...e.payload }),
  );
  try {
    for (const [i, file] of files.entries()) {
      current = i + 1;
      onProgress({ file: current, files: files.length, done: 0, total: 0 });
      try {
        const report = await COMMANDS.transfer.importFile({
          name: file.name.replace(/\.(csv|json)$/i, ""),
          text: await file.text(),
        });
        if (!summary.sources.includes(report.source)) summary.sources.push(report.source);
        summary.total += report.total;
        summary.liked += report.liked;
        summary.playlists += report.playlists;
        summary.albums += report.albums;
        summary.failures.push(...report.failures.map((f) => ({ ...f, file: file.name })));
        if (report.failures.length > 0) {
          summary.failedCsv += summary.failedCsv ? withoutHeader(report.failedCsv) : report.failedCsv;
        }
      } catch (e) {
        log.error(`couldn't import ${file.name}:`, e);
        summary.unreadable.push(`${file.name}: ${String(e)}`);
      }
    }
  } finally {
    (await off)();
    // an import adds to both, and nothing else tells the open screens
    await Promise.all([refreshPlaylists(), refreshLikes()]);
  }
  return summary;
}

/** Returns the path that was written, or null when the dialog was closed. */
export const saveCsv = (fileName: string, csv: string) => COMMANDS.transfer.saveCsv({ fileName, csv });

/** Saves the likes, playlists and albums as a CSV file. Returns the path, or null when the dialog was closed. */
export async function exportLibrary() {
  const csv = await COMMANDS.transfer.exportCsv();
  return saveCsv("yamp-library.csv", csv);
}
