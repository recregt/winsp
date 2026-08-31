import { highlightSegments } from "../highlight";

interface HighlightedTitleProps {
  text: string;
  matchedIndices: number[];
}

export function HighlightedTitle({
  text,
  matchedIndices,
}: HighlightedTitleProps) {
  const segments = highlightSegments(text, matchedIndices);
  return (
    <>
      {segments.map((segment, i) => (
        <span key={i} className={segment.highlighted ? "highlight" : undefined}>
          {segment.text}
        </span>
      ))}
    </>
  );
}
