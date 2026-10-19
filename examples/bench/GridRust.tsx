// `nsh trace examples/bench/GridRust.tsx`: the Grid screen with the Rust Button.
import { mount } from "nsh";
import { Button } from "nsh/ui";
import { Grid } from "./buttons";

mount(() => <Grid Button={Button} />);
