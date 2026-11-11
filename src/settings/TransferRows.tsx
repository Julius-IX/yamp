import { useRef, useState } from "react";
import { Box, Button, Dialog, DialogActions, DialogContent, DialogTitle, Typography } from "@mui/material";
import FileUploadOutlinedIcon from "@mui/icons-material/FileUploadOutlined";
import { logger } from "@/lib/log";
import { importFiles, saveCsv, type ImportProgress, type ImportSummary } from "@/lib/transfer";
import { useBackGuard } from "@/router/useBackGuard";
import { subdued } from "@/themes/ui";
import { ActionRow } from "./controls";

const log = logger("settings");

/** A long list of failures would make the dialog slow to open, the file has all of them. */
const SHOWN_FAILURES = 100;

const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

function ReportDialog({ summary, onClose }: { summary: ImportSummary | null; onClose: () => void }) {
  const [saved, setSaved] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useBackGuard(summary !== null, onClose);
  if (!summary) return null;

  const failed = summary.failures.length;
  const matched = summary.total - failed;
  const added = [
    summary.liked > 0 && plural(summary.liked, "liked song"),
    summary.playlists > 0 && plural(summary.playlists, "playlist"),
    summary.albums > 0 && plural(summary.albums, "album"),
  ].filter(Boolean);

  const download = async () => {
    setError(null);
    try {
      setSaved(await saveCsv("yamp-import-failed.csv", summary.failedCsv));
    } catch (e) {
      log.error("couldn't save the failed entries:", e);
      setError(String(e));
    }
  };

  return (
    <Dialog open onClose={onClose} fullWidth maxWidth="sm">
      <DialogTitle>Import finished</DialogTitle>
      <DialogContent>
        <Typography>
          {summary.total === 0
            ? "Nothing to import was found in the files."
            : `${matched} of ${plural(summary.total, "entry", "entries")} found in Yamp`}
          {summary.sources.length > 0 && ` (${summary.sources.join(", ")})`}
        </Typography>
        {added.length > 0 && <Typography sx={subdued}>Added {added.join(", ")}</Typography>}
        {summary.unreadable.map((u) => (
          <Typography key={u} color="error" variant="body2" sx={{ mt: 1 }}>{u}</Typography>
        ))}
        {failed > 0 && (
          <>
            <Typography sx={{ mt: 2, fontWeight: 600 }}>{plural(failed, "entry", "entries")} could not be added</Typography>
            <Box sx={{ maxHeight: 260, overflowY: "auto", mt: 1 }}>
              {summary.failures.slice(0, SHOWN_FAILURES).map((f, i) => (
                <Box key={i} sx={{ py: 0.5 }}>
                  <Typography variant="body2" noWrap>
                    {f.title}{f.artist && ` · ${f.artist}`}
                  </Typography>
                  <Typography variant="caption" sx={subdued}>
                    {f.reason}, {f.collection ? `${f.collection} in ` : ""}{f.file} row {f.row}
                  </Typography>
                </Box>
              ))}
              {failed > SHOWN_FAILURES && (
                <Typography variant="caption" sx={subdued}>
                  and {failed - SHOWN_FAILURES} more, they are all in the file
                </Typography>
              )}
            </Box>
            {saved && <Typography variant="body2" sx={{ mt: 1 }}>Saved to {saved}</Typography>}
            {error && <Typography variant="body2" color="error" sx={{ mt: 1 }}>{error}</Typography>}
          </>
        )}
      </DialogContent>
      <DialogActions>
        {failed > 0 && <Button onClick={() => void download()}>Download failed entries</Button>}
        <Button variant="contained" onClick={onClose}>Close</Button>
      </DialogActions>
    </Dialog>
  );
}

/**
 * The import and export of the library as CSV. Spotify and SoundCloud have no export of their
 * own that fits, tools like Exportify write the files that can be imported here.
 */
export default function TransferRows() {
  const picker = useRef<HTMLInputElement>(null);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [summary, setSummary] = useState<ImportSummary | null>(null);

  const run = async (files: File[]) => {
    if (files.length === 0) return;
    setProgress({ file: 1, files: files.length, done: 0, total: 0 });
    try {
      setSummary(await importFiles(files, setProgress));
    } catch (e) {
      log.error("the import stopped:", e);
    } finally {
      setProgress(null);
    }
  };

  let summaryText = "Add likes, playlists and albums from a CSV file, or the JSON files of the Spotify account data";
  if (progress) {
    const of = progress.files > 1 ? `file ${progress.file} of ${progress.files}, ` : "";
    summaryText = progress.total > 0 ? `Matching ${of}${progress.done} of ${progress.total}...` : `Reading ${of}...`;
  }

  return (
    <>
      <ActionRow
        title="Import library"
        icon={<FileUploadOutlinedIcon />}
        summary={summaryText}
        label="Import"
        busy={progress !== null}
        onAction={() => picker.current?.click()}
      />
      <input
        ref={picker}
        type="file"
        accept=".csv,.json,text/csv,application/json"
        multiple
        hidden
        onChange={(e) => {
          const files = Array.from(e.target.files ?? []);
          // the same file can be picked again after it was fixed
          e.target.value = "";
          void run(files);
        }}
      />
      <ReportDialog summary={summary} onClose={() => setSummary(null)} />
    </>
  );
}
