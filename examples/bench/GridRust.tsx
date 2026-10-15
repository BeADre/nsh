// `nsh trace examples/bench/GridRust.tsx`: the Grid screen with the Rust Button.
import { render, root } from "nsh";
import { Button } from "nsh/ui/mobile";
import { Grid } from "./buttons";

render(() => <Grid Button={Button} />, root);
