import { useParams } from "react-router-dom";
import type { Route, RouteParams } from "./paths";

/** The params of the matched route, typed from its pattern. */
export const useRouteParams = <P extends string>(_route: Route<P>) =>
  useParams() as RouteParams<P>;
