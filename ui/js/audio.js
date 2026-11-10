// Small WebAudio chime. No audio files, no network, no extra runtime weight.
// The audio context is primed on the first user gesture so autoplay policies
// do not silence the completion alert.

let context = null;

export function primeAudio() {
  try {
    if (!context) {
      const AudioContextClass = window.AudioContext || window.webkitAudioContext;
      if (!AudioContextClass) return;
      context = new AudioContextClass();
    }
    if (context.state === "suspended") void context.resume();
  } catch {
    // Audio is a nice-to-have; a failure must never break the timer.
  }
}

export function playChime() {
  primeAudio();
  if (!context) return;

  const start = context.currentTime + 0.03;
  const notes = [
    { frequency: 880.0, offset: 0.0, duration: 1.1, gain: 0.09 },
    { frequency: 1174.66, offset: 0.16, duration: 1.1, gain: 0.075 },
    { frequency: 1567.98, offset: 0.32, duration: 1.35, gain: 0.06 },
  ];

  for (const note of notes) {
    const at = start + note.offset;
    const oscillator = context.createOscillator();
    const envelope = context.createGain();
    const filter = context.createBiquadFilter();

    oscillator.type = "sine";
    oscillator.frequency.setValueAtTime(note.frequency, at);

    filter.type = "lowpass";
    filter.frequency.setValueAtTime(3200, at);

    envelope.gain.setValueAtTime(0.0001, at);
    envelope.gain.exponentialRampToValueAtTime(note.gain, at + 0.025);
    envelope.gain.exponentialRampToValueAtTime(0.0001, at + note.duration);

    oscillator.connect(filter);
    filter.connect(envelope);
    envelope.connect(context.destination);

    oscillator.start(at);
    oscillator.stop(at + note.duration + 0.05);
  }
}
