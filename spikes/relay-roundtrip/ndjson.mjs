export function createNdjsonParser(onFrame) {
  let buffered = "";

  return (chunk) => {
    buffered += chunk;

    while (true) {
      const newlineIndex = buffered.indexOf("\n");
      if (newlineIndex < 0) {
        return;
      }

      const line = buffered.slice(0, newlineIndex).trim();
      buffered = buffered.slice(newlineIndex + 1);
      if (line) {
        onFrame(JSON.parse(line));
      }
    }
  };
}

export function encodeFrame(frame) {
  return `${JSON.stringify(frame)}\n`;
}
