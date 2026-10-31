import {
  createContext, forwardRef, ReactElement, ReactNode, Ref, useCallback, useContext, useLayoutEffect,
  useMemo, useReducer, useRef,
} from "react";
import { Dialog, DialogProps, Slide } from "@mui/material";
import { TransitionProps } from "@mui/material/transitions";
import { useNavigate, useNavigationType, useOutlet } from "react-router-dom";

/**
 * Modals that live in the nested routes of a screen, never at the root of a tab.
 *
 * The screen is the parent route and renders `<TabModalOutlet />` where an `<Outlet />` would
 * go. Its modal routes are children that wrap their content in `<TabModal>`. The parent never
 * leaves, so the screen stays mounted and visible behind the modal with its state and scroll
 * position intact. A modal route at the top of a tab has no parent to stay behind.
 */

type TabModalState = {
  /** The route was popped. Kept rendered just long enough to play the exit animation. */
  closing: boolean;
  onExited: () => void;
  /** A TabModal reports itself here so the outlet knows it is holding a modal and not a page. */
  claim: (mounted: boolean) => void;
};

const TabModalContext = createContext<TabModalState>({
  closing: false,
  onExited: () => { },
  claim: () => { },
});

/**
 * Stand-in for `<Outlet />` in a screen that has modal child routes. A popped route is
 * not dropped at once, the last child stays rendered until its exit animation is done.
 */
export function TabModalOutlet() {
  const outlet = useOutlet();
  const [, rerender] = useReducer((n: number) => n + 1, 0);

  const held = useRef<ReactElement | null>(null);
  const modalMounted = useRef(false);
  const open = useRef(false);

  // Only a modal has an exit animation to wait for, any other child goes right away.
  if (outlet) held.current = outlet;
  else if (!modalMounted.current) held.current = null;
  open.current = !!outlet;

  const onExited = useCallback(() => {
    if (open.current) return; // opened again mid-animation, keep it
    held.current = null;
    rerender();
  }, []);
  const claim = useCallback((mounted: boolean) => { modalMounted.current = mounted; }, []);

  const closing = !outlet;
  const state = useMemo(() => ({ closing, onExited, claim }), [closing, onExited, claim]);

  if (!held.current) return null;
  return <TabModalContext.Provider value={state}>{held.current}</TabModalContext.Provider>;
}

const SlideUp = forwardRef(function SlideUp(
  props: TransitionProps & { children: ReactElement },
  ref: Ref<unknown>,
) {
  return <Slide direction="up" ref={ref} {...props} />;
});

export default function TabModal({
  children,
  slotProps,
  slots,
  sx,
  bottomSheet = false,
  ...props
}: { children: ReactNode; bottomSheet?: boolean } & Omit<DialogProps, "open" | "onClose">) {
  const navigate = useNavigate();
  const pushed = useNavigationType() === "PUSH";
  const { closing, onExited, claim } = useContext(TabModalContext);

  useLayoutEffect(() => {
    claim(true);
    return () => claim(false);
  }, [claim]);

  // Going back only works when the modal was pushed. Opened as the first entry of a stack
  // (a reset or a switch from another tab) there is nothing behind it, so go up to the parent.
  const close = () => (pushed ? navigate(-1) : navigate("..", { replace: true }));

  return (
    <Dialog
      {...props}
      disableScrollLock
      open={!closing}
      onClose={close}
      slots={{
        ...slots,
        ...(bottomSheet && { transition: SlideUp }),
      }}
      sx={[
        bottomSheet && {
          position: props.container ? "absolute" : undefined,
          "& .MuiDialog-container": { alignItems: "flex-end" },
          "& .MuiDialog-paper": {
            backgroundColor: "background.paper",
            m: 0,
            width: "100%",
            maxWidth: "none",
            maxHeight: "100%",
            borderBottomLeftRadius: 0,
            borderBottomRightRadius: 0,
          },
        },
        ...(Array.isArray(sx) ? sx : [sx]),
      ]}
      slotProps={{
        ...slotProps,
        backdrop: {
          ...(typeof slotProps?.backdrop === "function" ? {} : slotProps?.backdrop),
          transitionDuration: { enter: 0, exit: 200 },
        },
        transition: {
          ...(typeof slotProps?.transition === "function" ? {} : slotProps?.transition),
          onExited: (node: HTMLElement) => {
            const t = slotProps?.transition;
            if (typeof t !== "function") t?.onExited?.(node);
            onExited();
          },
        },
      }}
    >
      {children}
    </Dialog>
  );
}
