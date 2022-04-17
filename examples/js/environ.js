import { getenviron } from "std";

const environ = getenviron();
for (const [key, value] of Object.entries(environ)) {
    console.log(`${key}: ${value}`);
}
