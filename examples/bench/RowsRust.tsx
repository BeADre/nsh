// `nsh trace examples/bench/RowsRust.tsx`: the Rows screen with the Rust Button.
import { render, root } from "nsh";
import { Button } from "nsh/ui/mobile";
import { Rows } from "./buttons";

render(() => <Rows Button={Button} />, root);
