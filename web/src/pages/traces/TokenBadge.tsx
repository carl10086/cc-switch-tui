interface TokenBadgeProps {
  input: number;
  output: number;
}

export function TokenBadge({ input, output }: TokenBadgeProps) {
  return (
    <span className="inline-flex items-center gap-1 text-xs bg-muted px-2 py-0.5 rounded-full">
      <span className="text-blue-600">{input} in</span>
      <span className="text-muted-foreground">/</span>
      <span className="text-purple-600">{output} out</span>
    </span>
  );
}
