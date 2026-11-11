import { Button, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle, Link } from "@mui/material";
import { openUrl } from "@tauri-apps/plugin-opener";
import { PRIVACY_URL, TERMS_URL } from "@/lib/legal";
import { logger } from "@/lib/log";
import { isBoolean, useSetting } from "@/lib/settings";
import { useBackGuard } from "@/router/useBackGuard";

const log = logger("terms");

// a plain anchor would navigate the webview itself, so the link goes through the opener
function DocLink({ url, children }: { url: string; children: string }) {
  return (
    <Link
      component="button"
      type="button"
      sx={{ verticalAlign: "baseline" }}
      onClick={() => openUrl(url).catch((e) => log.error("couldn't open the link:", e))}
    >
      {children}
    </Link>
  );
}

/**
 * Shown on the first start until it is continued past. The acceptance is saved on this device
 * only. There is no `onClose` on purpose, so a backdrop click or Escape leaves it open. The
 * Android back press is held to keep it from navigating the tab behind.
 */
export default function TermsDialog() {
  const [accepted, setAccepted] = useSetting("termsAccepted", false, isBoolean);
  useBackGuard(!accepted, () => {});

  return (
    <Dialog open={!accepted} fullWidth maxWidth="xs">
      <DialogTitle>Before continuing</DialogTitle>
      <DialogContent>
        <DialogContentText>
          By continuing, you agree to the <DocLink url={TERMS_URL}>Terms of Service</DocLink> and
          the <DocLink url={PRIVACY_URL}>Privacy Policy</DocLink>.
        </DialogContentText>
      </DialogContent>
      <DialogActions>
        <Button variant="contained" onClick={() => setAccepted(true)}>Continue</Button>
      </DialogActions>
    </Dialog>
  );
}
