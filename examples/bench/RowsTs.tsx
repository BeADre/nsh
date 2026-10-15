// `nsh trace examples/bench/RowsTs.tsx`: the Rows screen with the Ts Button.
import { render, root } from "nsh";
import { Button } from "./TsButton";
import { Rows } from "./buttons";

render(() => <Rows Button={Button} />, root);
