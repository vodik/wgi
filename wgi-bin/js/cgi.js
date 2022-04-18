import { getenv } from "std";

export const method = () => getenv("REQUEST_METHOD");
export const userAgent = () => getenv("HTTP_USER_AGENT");
