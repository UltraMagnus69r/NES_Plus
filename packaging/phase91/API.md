# API configuration

Open UnMaker Lab, then AI Providers.

OpenAI is the first provider. Vision, image generation, and behavior assistance each have a model field. They can share one model or use different ones.

The key can be:

- Typed for this session only.
- Saved in the Secret Service (`secret-tool`, service `nesticles`, account `openai`) when that service is available.
- Read from `NESTICLES_OPENAI_API_KEY` when that variable is set.

Nesticles does not write this key into the project, a `.nasset` package, a `.nknowledge` file, or a log line. If Secret Service is missing, Save credentials leaves the key in memory and says so.

Test connection calls `https://api.openai.com/v1/models` with the key. An empty key is invalid credentials and does not claim a connection. Other results follow the HTTP status: invalid credentials, model not available, quota, rate limit, timeout, network, or a malformed body.

Image generation and vision stay off until you enable them. Sending an image asks once per session. Each session has a request limit. The ROM is not attached to those requests. Commercial character names are still refused by the existing image guard.

No price is calculated in the app.
