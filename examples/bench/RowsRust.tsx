// `nsh trace examples/bench/RowsRust.tsx`: the Rows screen with the Rust Button.
import { mount } from "nsh";
import { Button } from "nsh/ui";
import { Rows } from "./buttons";

mount(() => <Rows Button={Button} />);
