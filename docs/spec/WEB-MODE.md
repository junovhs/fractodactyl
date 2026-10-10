# Web mode: ChatGPT web writes the code, the agent orchestrates and lands

The owner sometimes asks for **web mode**. ChatGPT web has unlimited **Chat** use; the
agent's (Claude's) quota is the scarce resource. So ChatGPT, open in the owner's Chrome,
implements Ishoo issues and pushes branches. The agent picks the issue, sends the brief,
runs what only this machine can run, reviews, and lands through Ishoo. Use web mode only
when the owner asks for it in that session; otherwise the agent does the work itself.

Scripts: `tools/web-mode/` (`brief.py`, `send.sh`, `wait_push.sh`, `review.sh`, `head.txt`).
Set `WEB_DIR` to the session scratchpad; it holds briefs and messages.

## Rules (from the owner)

- **The agent does as little as possible.** No coding, no doc edits, no experiments in
  web mode: anything found in review goes back to ChatGPT. The agent orchestrates,
  verifies and runs the Ishoo landing. Exception: the owner says "take over".
- **Never hand-type long text.** Briefs come from the Ishoo store (`brief.py`), and replies
  are assembled by script from report files (grep the lines that matter). Keep the
  agent's context nearly empty. Look at screenshots only to check state, at reduced size.
- **Plain Chat mode, never "Work" mode.** Work mode spends the rate limit. The sidebar's
  "N% usage remaining" counter is about Work mode only; ignore it in Chat mode.
- **Leave the effort setting alone.** The owner sets it (Medium as of 2026-10-10).
- **One issue per chat.** When its branch lands, open a new chat for the next issue.
- **Few, big turns.** Each message to ChatGPT carries the full evidence (all results,
  all diagnostics) and asks for a complete fix with a test. Do not ask one question
  per round.
- **Shortest test that answers the question.** Before every run, ask what the shortest
  test is that gives everything needed, and run that, nothing longer. Example:
  rerun only the disputed frames (a path file with just those lines), not the 750-frame
  film. A long run is right only when the question needs it, e.g. one final regression
  run after a renderer change.
- **Heavy runs go to this machine** (Ryzen 9 3900X, 24 threads) when they are much
  faster here than on GitHub Actions. ChatGPT pushes the code and the command, the agent
  runs it and pastes the output back.

## The loop

1. **Pick and prepare.** `ishoo_status`, choose the issue, read it with `ishoo_show`. Fix
   the scope first if it is wrong or contradicts itself; `ishoo_edit` does that. AUTO-01
   said "out of scope: merging the research branch", which made landing impossible.
   Add any needed note to a file.
2. **Send.** `tools/web-mode/send.sh new ID [--note FILE]` opens chatgpt.com, clears any
   stale draft, pastes the brief and sends it. It prints the chat title. Check one
   reduced screenshot to confirm the brief was sent whole.
3. **Wait.** Run `tools/web-mode/wait_push.sh ID` in the background; it reports a push
   after a 10-minute settle, since ChatGPT often pushes several commits. A push is not
   "done": check that the chat shows "Worked for ..." with the final summary and no stop
   button before reviewing.
4. **Review.** `ishoo_start ID`, then `tools/web-mode/review.sh ID`, which merges
   `gpt/ID` into the worktree and runs `cargo test` and clippy. Check the issue's proof of
   done against what was pushed. If ChatGPT cites a GitHub Actions run, confirm with
   `gh run view` that it succeeded and ran on the landed code, or that later commits are
   docs only.
5. **Reply or land.** If something fails, send one complete message:
   `send.sh reply FILE`, with FILE assembled by script. Otherwise run `ishoo_resolve`
   (credit ChatGPT in What changed) and `ishoo_done`. Pull with `--ff-only` first. Then
   `git push origin --delete gpt/ID` and start the next issue with step 1.

## When to take over

If ChatGPT is on its third or fourth failing round, or the owner says so, stop sending
and do it yourself. Question the premise first: FIX-42 asked for de within 0.2% of
mpmath on samples 1e-14 px from the boundary. No f64 renderer can do that, and the
project never scores de/normal below 1e-3 px (compare.rs, BENC-04). Four ChatGPT fixes
chased a target that was outside the scoring contract. Check that the issue's target is
reachable and scored before asking for fixes. If the renderer is being restored, run
`git reset main` in the worktree so ChatGPT's intermediate commits do not land.

## Gotchas

- **Screen layout.** `send.sh` clicks fixed coordinates for the owner's layout: the
  composer is at (600, 982) on a new chat and at (600, 1990) in a running chat. If a
  screenshot shows a different layout, adjust them. Find the window with `wmctrl -l`,
  or set `WEB_WIN`.
- **Stale drafts.** The composer can keep a stale draft. `send.sh` clears it with
  ctrl+a and BackSpace before pasting.
- **Copying from ChatGPT.** Its code-block copy button does not reach `xclip`. Read
  commands from a full-resolution crop of the screenshot instead.
- **`koenigs_bench/run.sh` (path mode)** sends fd's stderr to log files under the output
  directory, not to the terminal. To find diagnostics, grep the logs.
- **`pkill -f PATTERN`** also matches the agent's own shell when the pattern appears in
  the command line, and kills it. Kill by PID, or use `pgrep -f "[p]attern"`.
- **Redirecting to `$dir/log`** fails when `$dir` does not exist yet. Run `mkdir` first.
- **`docs/spec/METHOD.md`** has mixed CRLF/LF line endings. In Python, use `newline=""`
  on both read and write.
