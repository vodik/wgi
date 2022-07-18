import * as os from "os"

export const handler = (event) => {
    console.log("Lambda function triggered!")
    console.log(`- Accepts: ${event.headers['accept']}`)
    console.log(`- User agent: ${event.headers['user-agent']}`)

    const message = event.queryStringParameters['message'] ?? 'Unset'
    const body = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Hello World</title>
</head>
<body>
  <h1>Hello World!</h1>
  <p>Lambda function triggered!
  <ul>
    <li>Http method: ${event.httpMethod}</li>
    <li>Host: ${event.headers['host']}</li>
    <li>User agent: ${event.headers['user-agent']}</li>
    <li>Query string test: ${message}</li>
  </ul>
  <!-- SAFETY: This is a demo, cut me some slack -->
  ${event.body}
</body>
</html>`

    os.sleep(3000)
    console.log(body)
    return {
        statusCode: 200,
        headers: {
            "Content-Type": ["text/html"],
        },
        body
    }
}
