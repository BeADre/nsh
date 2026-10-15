// `nsh trace examples/bench/GridTs.tsx`: the Grid screen with the Ts Button.
import { render, root } from "nsh";
import { Button } from "./TsButton";
import { Grid } from "./buttons";

render(() => <Grid Button={Button} />, root);
