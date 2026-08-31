export interface Segment {
  highlighted: boolean;
  text: string;
}

export function highlightSegments(
  text: string,
  matchedIndices: number[],
): Segment[] {
  const chars = Array.from(text);
  const isMatch = new Array(chars.length).fill(false);
  for (const i of matchedIndices) {
    if (i >= 0 && i < isMatch.length) {
      isMatch[i] = true;
    }
  }

  const segments: Segment[] = [];
  chars.forEach((ch, i) => {
    const highlighted = isMatch[i];
    const last = segments[segments.length - 1];
    if (last && last.highlighted === highlighted) {
      last.text += ch;
    } else {
      segments.push({ highlighted, text: ch });
    }
  });
  return segments;
}
