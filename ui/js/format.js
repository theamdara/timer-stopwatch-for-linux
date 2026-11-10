// Pure formatting helpers shared by the UI. No DOM access lives here, which
// keeps the display rules easy to reason about (and to test separately).

export function pad2(value) {
  return String(Math.max(0, Math.trunc(value))).padStart(2, "0");
}

export function clockDisplay(nowMs, config) {
  const date = new Date(nowMs);
  const hours24 = date.getHours();
  const minutes = date.getMinutes();
  const seconds = date.getSeconds();
  const use24h = Boolean(config.use_24h);

  let hourText;
  let suffix = "";
  if (use24h) {
    hourText = pad2(hours24);
  } else {
    const hours12 = hours24 % 12 || 12;
    hourText = pad2(hours12);
    suffix = hours24 >= 12 ? "PM" : "AM";
  }

  let main = `${hourText}:${pad2(minutes)}`;
  if (config.show_seconds) main += `:${pad2(seconds)}`;

  let metaLeft = "Local time";
  if (config.show_date) {
    metaLeft = new Intl.DateTimeFormat(undefined, {
      weekday: "short",
      month: "short",
      day: "numeric",
    }).format(date);
  }

  const metaRight = [
    use24h ? "24-hour" : "12-hour",
    config.show_seconds ? "seconds" : "no seconds",
  ].join(" · ");

  return { main, suffix, metaLeft, metaRight };
}

export function countdownDisplay(remainingMs) {
  const totalSeconds = Math.max(0, Math.ceil(remainingMs / 1000));
  return { main: durationClock(totalSeconds) };
}

export function stopwatchDisplay(elapsedMs) {
  const centiseconds = Math.floor(Math.max(0, elapsedMs) / 10);
  const hundredths = centiseconds % 100;
  const totalSeconds = Math.floor(centiseconds / 100);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  const main =
    hours > 0
      ? `${hours}:${pad2(minutes)}:${pad2(seconds)}`
      : `${pad2(minutes)}:${pad2(seconds)}`;

  return { main, suffix: `.${pad2(hundredths)}` };
}

export function pomodoroDisplay(remainingMs) {
  const totalSeconds = Math.max(0, Math.ceil(remainingMs / 1000));
  return { main: durationClock(totalSeconds) };
}

function durationClock(totalSeconds) {
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  return hours > 0
    ? `${hours}:${pad2(minutes)}:${pad2(seconds)}`
    : `${pad2(minutes)}:${pad2(seconds)}`;
}

export function splitDuration(totalSeconds) {
  const seconds = Math.max(0, Math.trunc(totalSeconds));
  return {
    hours: Math.floor(seconds / 3600),
    minutes: Math.floor((seconds % 3600) / 60),
    seconds: seconds % 60,
  };
}

export function humanDuration(totalSeconds) {
  const seconds = Math.max(0, Math.trunc(totalSeconds));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = seconds % 60;

  const parts = [];
  if (hours > 0) parts.push(`${hours} hr`);
  if (minutes > 0) parts.push(`${minutes} min`);
  if (rest > 0 && hours === 0) parts.push(`${rest} sec`);
  return parts.length ? parts.join(" ") : "0 sec";
}

export function phaseLabel(phase) {
  switch (phase) {
    case "short_break":
      return "Short break";
    case "long_break":
      return "Long break";
    default:
      return "Focus";
  }
}
