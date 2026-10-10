<!-- ishoo:begin -->
This repository is managed by Ishoo. Before handling the first user request, call the `ishoo_brief` MCP tool, and drive all issue, plan, and decision work through the `ishoo_*` MCP tools, never the Ishoo command-line interface. If those tools fail, retry: a dropped connection usually recovers on the next call. If Ishoo stays unavailable, never touch `main`: commit your work on a branch and push nothing (in a cloud session where Ishoo is not installed, push only that branch), then tell the user Ishoo must be enabled.
<!-- ishoo:end -->

Web mode: when the owner asks for "web mode", ChatGPT web writes the code and you orchestrate and land. Follow docs/spec/WEB-MODE.md and its scripts in tools/web-mode/.
