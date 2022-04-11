import { getenv } from "std";

const userAgent = getenv("HTTP_USER_AGENT");
console.log(`Hello ${userAgent} - from QuickJS`);
