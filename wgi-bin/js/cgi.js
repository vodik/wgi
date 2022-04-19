import { getenv } from "std";

export const method = () => getenv("REQUEST_METHOD");
export const userAgent = () => getenv("HTTP_USER_AGENT");

export const getQuery = () => {
    const query_string = getenv("QUERY_STRING");
    return Object.fromEntries(query_string.split("&").map((arg) => arg.split("=")))
};
