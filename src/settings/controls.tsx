import { useState, type ReactNode } from "react";
import {
  Button, Chip, CircularProgress, Dialog, DialogActions, DialogContent, DialogTitle, FormControlLabel, MenuItem, Radio,
  RadioGroup, Select, Switch,
} from "@mui/material";
import { useBackGuard } from "@/router/useBackGuard";
import { IS_MOBILE } from "@/util/platform";
import { PrefRow } from "./rows";

export function SwitchRow({ title, summary, checked, onChange }: {
  title: string;
  summary?: string;
  checked: boolean;
  onChange: (on: boolean) => void;
}) {
  return (
    <PrefRow
      title={title}
      summary={summary}
      onClick={IS_MOBILE ? () => onChange(!checked) : undefined}
      control={
        <Switch
          checked={checked}
          onChange={(_, on) => onChange(on)}
          // the mobile row toggles as well, so a press on the switch itself must not reach it
          onClick={(e) => e.stopPropagation()}
        />
      }
    />
  );
}

export type Choice<T extends string> = { value: T; label: string };

/** A menu on desktop. On mobile the row shows the current choice and opens a radio dialog. */
export function ChoiceRow<T extends string>({ title, summary, value, options, onChange }: {
  title: string;
  summary?: string;
  value: T;
  options: Choice<T>[];
  onChange: (value: T) => void;
}) {
  const [open, setOpen] = useState(false);
  useBackGuard(open, () => setOpen(false));

  if (!IS_MOBILE) {
    return (
      <PrefRow
        title={title}
        summary={summary}
        control={
          <Select
            size="small"
            value={value}
            onChange={(e) => onChange(e.target.value as T)}
            sx={{
              minWidth: 160, fontSize: "0.875rem", bgcolor: "action.hover",
              "& .MuiOutlinedInput-notchedOutline": { borderColor: "divider" },
            }}
          >
            {options.map((o) => (
              <MenuItem key={o.value} value={o.value}>{o.label}</MenuItem>
            ))}
          </Select>
        }
      />
    );
  }
  return (
    <>
      <PrefRow title={title} summary={options.find((o) => o.value === value)?.label} onClick={() => setOpen(true)} />
      <Dialog open={open} onClose={() => setOpen(false)} fullWidth maxWidth="xs">
        <DialogTitle>{title}</DialogTitle>
        <DialogContent sx={{ px: 1 }}>
          <RadioGroup
            value={value}
            onChange={(_, v) => {
              onChange(v as T);
              setOpen(false);
            }}
          >
            {options.map((o) => (
              <FormControlLabel
                key={o.value}
                value={o.value}
                label={o.label}
                control={<Radio />}
                sx={{ mx: 0, px: 1.5, py: 0.5 }}
              />
            ))}
          </RadioGroup>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setOpen(false)}>Cancel</Button>
        </DialogActions>
      </Dialog>
    </>
  );
}

/** An option that is planned but not built. It holds its place in the list and cannot be used. */
export function PlaceholderRow({ title, summary }: { title: string; summary: string }) {
  return <PrefRow title={title} summary={summary} control={<Chip size="small" variant="outlined" label="Coming later" />} />;
}

/** The pill button of a desktop row. Sized and colored the same everywhere a row has one. */
export function ActionButton({ label, busy = false, danger = false, disabled = false, onClick }: {
  label: string;
  busy?: boolean;
  danger?: boolean;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <Button
      size="small"
      variant="outlined"
      color={danger ? "error" : "inherit"}
      disabled={busy || disabled}
      startIcon={busy ? <CircularProgress size={14} /> : undefined}
      onClick={onClick}
      sx={{ px: 2.5, textTransform: "none", fontWeight: 600, borderColor: danger ? undefined : "divider" }}
    >
      {label}
    </Button>
  );
}

/** A button on desktop, a pressable row on mobile. */
export function ActionRow({ title, icon, summary, label, busy = false, danger = false, disabled = false, onAction }: {
  title: string;
  icon?: ReactNode;
  summary?: string;
  /** Text of the desktop button. */
  label: string;
  busy?: boolean;
  danger?: boolean;
  disabled?: boolean;
  onAction: () => void;
}) {
  if (IS_MOBILE) {
    return (
      <PrefRow
        title={title}
        icon={icon}
        summary={summary}
        danger={danger}
        onClick={busy || disabled ? undefined : onAction}
        control={busy ? <CircularProgress size={20} /> : undefined}
      />
    );
  }
  return (
    <PrefRow
      title={title}
      icon={icon}
      summary={summary}
      danger={danger}
      control={<ActionButton label={label} busy={busy} danger={danger} disabled={disabled} onClick={onAction} />}
    />
  );
}
