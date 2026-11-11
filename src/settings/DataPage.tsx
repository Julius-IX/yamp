import { useEffect, useState } from "react";
import { Box, Button, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle, Stack } from "@mui/material";
import FolderOutlinedIcon from "@mui/icons-material/FolderOutlined";
import { open } from "@tauri-apps/plugin-dialog";
import { COMMANDS } from "@/lib/commands";
import { useLocalVersion } from "@/lib/local";
import { logger } from "@/lib/log";
import { cancelSignIn, loadSync, signIn, signOut, syncNow, useSyncState } from "@/lib/sync";
import type { LocalUsage, SyncProvider } from "@/lib/types";
import { useBackGuard } from "@/router/useBackGuard";
import { IS_MOBILE } from "@/util/platform";
import { ActionButton, ActionRow } from "./controls";
import FolderPicker from "./FolderPicker";
import { providerIcon } from "./icons";
import { PrefRow, Section } from "./rows";
import StatsRows from "./StatsRows";
import TransferRows from "./TransferRows";

const log = logger("settings");

const size = (bytes: number) => {
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let n = bytes;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i++;
  }
  return `${i === 0 ? n : n.toFixed(1)} ${units[i]}`;
};

const songs = (n: number) => `${n} ${n === 1 ? "song" : "songs"}`;
const totals = (count: number, bytes: number) => `${songs(count)}, ${size(bytes)}`;

/** Read again whenever a download lands or goes, or a scan of the folders finishes. */
function useLocalUsage() {
  const version = useLocalVersion();
  const [usage, setUsage] = useState<LocalUsage | null>(null);
  useEffect(() => {
    COMMANDS.local
      .usage()
      .then(setUsage)
      .catch((e) => log.error("couldn't read the size of the local songs:", e));
  }, [version]);
  return usage;
}

function useFolders() {
  const version = useLocalVersion();
  const [folders, setFolders] = useState<string[]>([]);
  useEffect(() => {
    COMMANDS.local
      .dirs()
      .then(setFolders)
      .catch((e) => log.error("couldn't list the music folders:", e));
  }, [version]);
  return [folders, setFolders] as const;
}

const folderName = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() ?? path;

/**
 * The folders whose audio files show up in the local songs. Removing one only forgets it, the
 * files stay where they are.
 */
function FolderRows() {
  const [folders, setFolders] = useFolders();
  const [removing, setRemoving] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  useBackGuard(removing !== null, () => setRemoving(null));

  const addPath = async (path: string) => {
    setError(null);
    try {
      await COMMANDS.local.addDir({ path });
      setFolders(await COMMANDS.local.dirs());
    } catch (e) {
      log.error("couldn't add the music folder:", e);
      setError(String(e));
    }
  };

  const add = async () => {
    setError(null);
    try {
      if (IS_MOBILE) {
        // a phone has no folder dialog that gives a path, and the folders can only be read once
        // the permission for audio files is given
        if (await COMMANDS.local.requestAccess()) setPicking(true);
        else setError("Allow access to audio files for this app in the system settings, then try again");
        return;
      }
      const picked = await open({ directory: true, multiple: false, title: "Add music folder" });
      if (typeof picked === "string") await addPath(picked);
    } catch (e) {
      log.error("couldn't add the music folder:", e);
      setError(String(e));
    }
  };

  const remove = async () => {
    const path = removing;
    setRemoving(null);
    if (path === null) return;
    try {
      await COMMANDS.local.removeDir({ path });
      setFolders(await COMMANDS.local.dirs());
    } catch (e) {
      log.error(`couldn't remove ${path}:`, e);
    }
  };

  return (
    <>
      <ActionRow
        title="Music folders"
        icon={<FolderOutlinedIcon />}
        summary={error ?? "Songs in these folders show up in Local, new files are picked up by themselves"}
        label="Add folder"
        onAction={() => void add()}
      />
      {folders.map((path) => (
        <ActionRow
          key={path}
          title={folderName(path)}
          summary={path}
          label="Remove"
          danger
          onAction={() => setRemoving(path)}
        />
      ))}
      {picking && (
        <FolderPicker
          onClose={() => setPicking(false)}
          onPick={(path) => {
            setPicking(false);
            void addPath(path);
          }}
        />
      )}
      <Dialog open={removing !== null} onClose={() => setRemoving(null)} fullWidth maxWidth="xs">
        <DialogTitle>Remove folder</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {removing ? `The songs of "${folderName(removing)}" leave the library. The files stay where they are.` : ""}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setRemoving(null)}>Cancel</Button>
          <Button color="error" variant="contained" onClick={() => void remove()}>Remove</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}

/**
 * One cloud service. Signing in links the library to the account and the sync then runs by itself,
 * see `lib/sync`. Only one service can be linked at a time, since devices have to share the same one.
 */
function ProviderRow({ provider }: { provider: SyncProvider }) {
  const sync = useSyncState();
  const [menu, setMenu] = useState(false);
  useBackGuard(menu, () => setMenu(false));

  const { status } = sync;
  const linked = !!status?.connected && status.provider === provider.id;
  const elsewhere = !!status?.connected && !linked;
  const waiting = sync.signingIn === provider.id;
  const blocked = elsewhere || (sync.signingIn !== null && !waiting);

  let summary = "Not linked";
  if (waiting) summary = "Waiting for the browser...";
  else if (linked) {
    const when = sync.lastSyncMs && new Date(sync.lastSyncMs).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    summary = `Linked as ${status?.account ?? "your account"}${sync.syncing ? ", syncing..." : when ? `, synced ${when}` : ""}`;
  } else if (elsewhere) summary = "Sign out of the linked service first";

  if (!IS_MOBILE) {
    return (
      <PrefRow
        title={provider.name}
        icon={providerIcon(provider.id)}
        summary={summary}
        control={
          waiting ? (
            <ActionButton label="Cancel" onClick={cancelSignIn} />
          ) : linked ? (
            <Stack direction="row" spacing={1}>
              <ActionButton label="Sync now" busy={sync.syncing} onClick={syncNow} />
              <ActionButton label="Sign out" danger onClick={signOut} />
            </Stack>
          ) : (
            <ActionButton label="Sign in" disabled={blocked} onClick={() => signIn(provider.id)} />
          )
        }
      />
    );
  }

  const press = waiting ? cancelSignIn : linked ? () => setMenu(true) : blocked ? undefined : () => signIn(provider.id);
  return (
    <>
      <PrefRow title={provider.name} icon={providerIcon(provider.id)} summary={summary} onClick={press} />
      <Dialog open={menu} onClose={() => setMenu(false)} fullWidth maxWidth="xs">
        <DialogTitle>{provider.name}</DialogTitle>
        <DialogContent>
          <DialogContentText>{summary}</DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setMenu(false)}>Close</Button>
          <Button color="error" onClick={() => (setMenu(false), signOut())}>Sign out</Button>
          <Button onClick={() => (setMenu(false), syncNow())}>Sync now</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}

export default function DataPage() {
  const sync = useSyncState();
  const usage = useLocalUsage();
  useEffect(() => {
    void loadSync();
  }, []);

  return (
    <Section title="Data" cloud="data">
      <PrefRow
        title="Downloaded Songs"
        summary={
          usage ? (
            <>
              {totals(usage.downloadedSongs + usage.folderSongs, usage.downloadedBytes + usage.folderBytes)}
              <Box component="span" sx={{ display: "block" }}>
                Downloaded in the app: {totals(usage.downloadedSongs, usage.downloadedBytes)}
              </Box>
              <Box component="span" sx={{ display: "block" }}>
                In music folders: {totals(usage.folderSongs, usage.folderBytes)}
              </Box>
            </>
          ) : (
            "Checking..."
          )
        }
      />
      <FolderRows />
      {sync.providers.map((p) => (
        <ProviderRow key={p.id} provider={p} />
      ))}
      {/* a build made without any OAuth client has nothing to sign in to */}
      {sync.loaded && sync.providers.length === 0 && (
        <PrefRow title="Cloud sync" summary="No cloud services are set up in this build" />
      )}
      <TransferRows />
      <StatsRows />
    </Section>
  );
}
