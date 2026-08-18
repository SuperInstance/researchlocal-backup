# Captain's Guide — F/V EILEEN

For Casey. Plain words, fishing words. Keep this in the pilot house.

---

## 1. What this thing does

You hired a deckhand who never sleeps, never complains, and never touches the wheel without asking. He watches the sounder and the screen all day and writes down what he sees — that part works today. When he's sure, he suggests. When he's not, he asks — that part's still on the bench. The wheel is still yours. Always.

## 2. Your day with it

**Morning.** You start the boat. The cascade has been running all night as it should — watching yesterday's captures, writing its notes and records, pulling tide and weather for the grounds. That part works today. The dial that picks how much rope he gets is still on the bench; right now he watches and writes, full stop.

**Setting out.** You drive. He's already watching the screen — every ten minutes a frame goes into the book, time-stamped and position-stamped, no effort from you. This is the live part. The piece where he listens to your voice and learns your words — "heading for the rock," "birds working the edge" — is rigged but not baited yet. It's coming.

**Fishing.** You drive. He watches the screen behind you. Every ten minutes he writes a short record of what the sounder showed — schools, bottom, water. This works today. Once an hour he rolls those notes into a one-screen briefing. The briefing gets written; the part that pings your phone is waiting on Telegram creds. Until then, the briefings live on the boat — open the Debriefs page from the scrubber.

**You never have to:** save logs, name files, check storage, sync anything, or explain what you saw. He does that — and this works today.

**Evening.** You tie up. He keeps thinking for a few more minutes — one last pass over the day's pictures so nothing gets tossed he should've kept. That pass is calibrated and live; it keeps a frame if any one of three reasons says so. Then he rests until you start again.

## 3. The dial

One knob. Four positions. Visible from the helm, honest about what the boat is actually doing. **The knob itself is coming — today he runs at Log, watches and writes, no hands on anything.** What follows is what each position means when the dial is wired.

**0 — Log.** He watches. He writes. He does not steer or throttle. Use this when you're teaching him a new spot, or when something's off and you want his notes without his hands.

**1 — Coach.** He suggests in plain words on the dash. "Looks like feed on the bottom, 35 fathoms, 2 o'clock." You do the driving. He'll also *say back* what he would do — agree or disagree with you. Use this when you're learning each other.

**2 — Supervise.** He drives within safe limits. Same kind of move you've already approved? He just does it. New situation? He asks first. Use this once you trust him on the basics.

**3 — Autopilot.** He drives within safe limits on his own. He only asks when something's new, weird, or unsure. Use this when you've worked together a long time and the agreement rate is high.

After any restart, he comes back at watch-and-write. The plan is for the dial to come back at **1 — Coach** once the dial exists; today the floor is lower and that's the point.

## 4. The scrubber

The scrubber is the playback of your day — sounder, his notes, your track, all on one timeline at the bottom of the screen. Open it at `http://localhost:8080`. This is the live surface.

It doesn't open on a blank bar. It puts the cursor on the day's highlight — the school, the bottom change, whatever he flagged as the most interesting minute — with a one-line caption at the top. You did nothing and already know what happened today.

Drag the timeline to scrub. Drag your finger *down* toward the bottom edge to slow down — 12-hour sweep up top, frame-precise down low. One gesture, both speeds.

Big buttons. Drag the screen, not menus. If the screen is wet, the keyboard works too: **left/right** steps a frame (**shift + left/right** jumps ten minutes), **space** plays and pauses, **[** and **]** jump to the previous or next event, **1/2/3** is 1×, 2×, 10× playback, **H** jumps back to the highlight. Every screen button has a key.

Up top, the clipboard button (📋) opens a second page — **Debriefs** — the day's briefings in plain English, dated and stacked. That's where his hourly summaries and the morning's tide and weather live. Works today.

It remembers where you left off — cursor, layers, speed. Close it, come back tomorrow, same spot.

**Tip:** a frame at the cursor loads in under a tenth of a second from warm; the whole day, under a third. If it stutters, that's a problem — tell me.

## 5. Telegram briefings

Your phone is meant to get three things, on a schedule:

- **Hourly summary.** What's on the screen, what he noticed, anything new. Short.
- **Trip digest.** When you tie up. The whole day, one screen.
- **The ask.** Only when he needs you. Never more than once every ten minutes unless it's important.

He writes all three every cycle — the hourly summary and the trip digest are there now, readable on the Debriefs page in the scrubber. The piece that pushes them to your phone over Telegram is wired but waiting on credentials (BotFather token, chat ID). Set those and the same briefings ship to your pocket without any other change.

If you don't want phone pings at sea, leave the creds unset. He won't bother you.

## 6. When he asks you something

An ask is a **card**, not an alarm — and this part is rigged but not baited yet. The envelope is in the kernel; the card itself isn't on the screen. When it ships, it shows up with the facts first — what he saw, why he's unsure — and his recommendation behind a tap.

**The card has:**
- The question, in plain words.
- Two or three choices, each with what happens if you pick it.
- A countdown. If you do nothing, the **safe default** kicks in. The boat never waits on you.
- A confidence bar showing "17 of my last 20 calls like this held" — not a fake percent.

**If you ignore it:** nothing bad happens. The safe default runs, and a little badge stays on the card until you read it. The boat keeps fishing.

**Common cards:**
- "Wind shifted, gains are chattering. Switch to rough-water steering? *(defaults to No in 1:47)*"
- "Feed marks at 30fm, birds working. Set up a pass? *(defaults to Hold)*"
- "Mark went quiet last 10 minutes. Keep watching or move? *(defaults to Keep watching)*"

## 7. When something's wrong

**The boat's safety rules are hard-coded.** He can't change them, you can't turn them off, and no bug can outrank the wheel in your hand.

**He will, on his own:**
- Restart his own watcher if his heartbeat goes stale — a watchdog ticks every two minutes and brings the cascade back if it trips. Works today.
- Drop back to a safer dial if a sensor quits. Coming — there's no dial yet to drop back from.
- Refuse to call a backup good unless every file re-hashes clean after the copy. Works today — `VERIFIED` means verified, silent rot caught.
- Keep watching and writing notes, even if the rest of the network is down. Works today — it's all local.

**He will NEVER, on his own:**
- Drive without you at the wheel being in command.
- Override you grabbing the wheel or the throttle. Your hand wins. Every time.
- Delete a day's record without reading it once.
- Push your boat's data anywhere without you saying so first.
- Pretend he knows when he doesn't.

If you grab the wheel, he goes quiet. The plan is for him to *politely ask why* out loud — even a grunt is teaching him. That voice loop is rigged but not baited yet. Today he just keeps writing.

## 8. One-tap buttons

Five buttons on the deck screen. One tap each. He stamps them with where, when, and how deep automatically. This whole screen is coming — deck mode isn't built yet. What each button will do:

- **Mark School.** Drops a flag on the screen at this spot, this depth.
- **Haul.** Snapshots the sounder and the moment. Asks the count when you come back.
- **Gear Issue.** Saves the last minute of screen and sensor. Flags it for review. No jargon.
- **Bycatch.** Freezes the spot, chimes once, queues a photo prompt.
- **End of Set.** Closes the chunk, writes a short catch-and-effort summary.

**PIN-locked stuff** (gain, frequency, thresholds): not on the deck screen. That's a different door.

## 9. Your words

The plan is for him to learn your words by listening while you drive. Say what you see, out loud, like you would to a deckhand. The voice pipeline is rigged but not baited yet — he can't hear you today.

- "Feed layer." → he ties that to the kind of marks you mean.
- "The rock." → he ties that to the GPS spot you call by that name.
- "Working birds." → he learns what you mean.

When he says something back that isn't quite your word, correct him out loud. He'll try again.

**One rule:** he won't make up names. If he doesn't know what you mean, he marks it *unclassified* and asks later — usually in the evening digest.

## 10. What he's learning

Over time, three things get better:

1. **His eyes.** The retention pass that decides what's worth keeping is calibrated and live — novel frames he keeps are training data for next season. The part where you tap MARK SCHOOL to label a school is coming with deck mode.
2. **His words.** The more you talk, the more of your vocab he owns. New terms get proposed in your weekly digest. The vocab memory is wired; the voice ear that feeds it isn't.
3. **His judgment.** Every time you say yes to a class of ask, he notices. After enough yeses, that kind of ask stops interrupting — it just runs. After enough noes, he bothers you less with that one. Coming — needs the ask cards first.

**The number that matters:** weekly, the debrief will show how often you two agreed this trip, and — more importantly — how often *he* was right the times you disagreed. That number, not the dial, is what moves him up the ladder. The weekly card is coming; the daily debrief page is there now.

Slow is fine. The boat is yours.

---

*If something on here doesn't match what the boat does, the boat is right and this guide is wrong. Tell me and I'll fix the guide.*