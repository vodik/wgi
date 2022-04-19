import { getQuery } from "./cgi.js";
import { fibonacci } from "./math.js";

const query = getQuery();
console.log(fibonacci(query.t || 50));
