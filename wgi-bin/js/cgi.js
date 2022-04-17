import { getenv } from "std";

export const userAgent = () => getenv("HTTP_USER_AGENT");
