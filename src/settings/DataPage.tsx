import { useEffect, useState } from "react";
import { Button, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle, Stack } from "@mui/material";
import { COMMANDS } from "@/lib/commands";
import { logger } from "@/lib/log";
import { cancelSignIn, loadSync, signIn, signOut, syncNow, useSyncState } from "@/lib/sync";
import type { SyncProvider } from "@/lib/types";
import { useBackGuard } from "@/router/useBackGuard";
import { IS_MOBILE } from "@/util/platform";
import { ActionButton } from "./controls";
import { providerIcon } from "./icons";
import { PrefRow, Section } from "./rows";

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

function useDownloadUsage() {
  const [usage, setUsage] = useState<{ songs: number; bytes: number } | null>(null);
  useEffect(() => {
    Promise.all([COMMANDS.downloads.count(), COMMANDS.downloads.totalSize()])
      .then(([songs, bytes]) => setUsage({ songs, bytes }))
      .catch((e) => log.error("couldn't read the download size:", e));
  }, []);
  return usage;
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
  const usage = useDownloadUsage();
  useEffect(() => {
    void loadSync();
  }, []);

  return (
    <Section title="Data" mobileHeading={false}>
      <PrefRow
        title="Downloaded Songs"
        summary={usage ? `${usage.songs} ${usage.songs === 1 ? "song" : "songs"}, ${size(usage.bytes)}` : "Checking..."}
      />
      {sync.providers.map((p) => (
        <ProviderRow key={p.id} provider={p} />
      ))}
      {/* a build made without any OAuth client has nothing to sign in to */}
      {sync.loaded && sync.providers.length === 0 && (
        <PrefRow title="Cloud sync" summary="No cloud services are set up in this build" />
      )}
    </Section>
  );
}
