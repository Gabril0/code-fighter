function inline(text, key) {
  const parts = String(text).split(/(\*\*[^*]+\*\*|`[^`]+`)/g);
  return parts.map((part, index) => {
    if (part.startsWith("**") && part.endsWith("**")) {
      return <strong key={`${key}-${index}`}>{part.slice(2, -2)}</strong>;
    }
    if (part.startsWith("`") && part.endsWith("`")) {
      return <code key={`${key}-${index}`}>{part.slice(1, -1)}</code>;
    }
    return part;
  });
}

export default function Statement({ markdown }) {
  const blocks = [];
  const lines = String(markdown ?? "").split("\n");
  let fence = null;
  let bullets = null;

  const flushBullets = () => {
    if (!bullets) return;
    blocks.push(
      <ul key={`ul-${blocks.length}`}>
        {bullets.map((item, index) => (
          <li key={index}>{inline(item, `li-${blocks.length}-${index}`)}</li>
        ))}
      </ul>,
    );
    bullets = null;
  };

  lines.forEach((line, index) => {
    if (line.trim().startsWith("```")) {
      if (fence) {
        blocks.push(<pre key={`pre-${index}`}>{fence.join("\n")}</pre>);
        fence = null;
      } else {
        flushBullets();
        fence = [];
      }
      return;
    }
    if (fence) {
      fence.push(line);
      return;
    }

    const heading = line.match(/^(#{1,4})\s+(.*)$/);
    if (heading) {
      flushBullets();
      const Tag = `h${Math.min(5, heading[1].length + 2)}`;
      blocks.push(<Tag key={`h-${index}`}>{inline(heading[2], `h-${index}`)}</Tag>);
      return;
    }

    const bullet = line.match(/^\s*[-*]\s+(.*)$/);
    if (bullet) {
      bullets = bullets ?? [];
      bullets.push(bullet[1]);
      return;
    }

    flushBullets();
    if (line.trim()) {
      blocks.push(<p key={`p-${index}`}>{inline(line.replace(/^>\s?/, ""), `p-${index}`)}</p>);
    }
  });

  flushBullets();
  if (fence) blocks.push(<pre key="pre-tail">{fence.join("\n")}</pre>);

  return <div className="statement">{blocks}</div>;
}
