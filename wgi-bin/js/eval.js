import { evalScript, in as stdin } from "std";
import { method } from "./cgi.js";

if (method() !== "POST") {
    console.log("Status: 405\n\n");
} else {
    const input = stdin.readAsString()
    evalScript(input);
}
