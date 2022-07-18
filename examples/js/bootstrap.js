import { nextEvent, sendResponse } from "lambda"
import { handler } from "./lambda.js"

const event = nextEvent()
sendResponse(handler(event))
