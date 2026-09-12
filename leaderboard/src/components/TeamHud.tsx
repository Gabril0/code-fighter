import { initials } from "../lib/initials";

function Member({ member, fighting, tag, color }) {
  return (
    <div className={fighting ? "member member--fighting" : "member"}>
      <div className="member-photo" style={{ borderColor: color }}>
        {member.photo ? <img src={member.photo} alt="" /> : null}
        <span className="member-initials">{initials(member.name)}</span>
      </div>
      <span className="member-name">{member.name}</span>
      {tag && <span className="member-tag">{tag}</span>}
    </div>
  );
}

function Rung({ question, position, done, live, tries, color }) {
  const classes = ["rung"];
  if (done) classes.push("rung--done");
  else if (live) classes.push("rung--live");
  else classes.push("rung--locked");

  const label = tries === 1 ? "1 attempt" : `${tries} attempts`;

  return (
    <span
      className={classes.join(" ")}
      style={done || live ? ({ "--team": color } as React.CSSProperties) : undefined}
      title={`${question.title}: ${tries === 0 ? "no attempts" : label}`}
    >
      <em className="rung-slot">Q{position}</em>
      <strong className="rung-value">{done ? question.points : live ? "-" : "🔒"}</strong>
      {tries > 0 && !done && (
        <i className="rung-tries" key={tries}>
          {tries}
        </i>
      )}
    </span>
  );
}

export default function TeamHud({ team, questions, leading, falls, outcome }) {
  if (!team) return null;

  const champion = outcome === team.side;
  const decided = Boolean(outcome);

  const solvedCount = Object.keys(team.solves ?? {}).length;
  const totalTries: number = Object.values<any>(team.attempts ?? {}).reduce((sum: number, count: any) => sum + count, 0);
  const firstUnsolved = questions.findIndex((question) => !team.solves?.[question.id]);
  const openThrough = firstUnsolved === -1 ? questions.length - 1 : firstUnsolved;

  return (
    <div className={`hud hud--${team.side}${leading ? " hud--leading" : ""}`}>
      <div className="hud-title" style={{ color: team.color }}>
        {team.icon && <img className="hud-icon" src={team.icon} alt="" />}
        <span className="hud-name">{team.name}</span>
      </div>

      <div className="hud-members">
        {team.members.map((member, index) => {
          const fighting =
            !decided && team.members.length > 0 && index === falls % team.members.length;
          return (
            <Member
              key={member.id}
              member={member}
              color={team.color}
              fighting={fighting || champion}
              tag={champion ? "CHAMPION" : fighting ? "IN THE RING" : null}
            />
          );
        })}
      </div>

      <div className="hud-ladder">
        {questions.map((question, index) => (
          <Rung
            key={question.id}
            question={question}
            position={index + 1}
            done={Boolean(team.solves?.[question.id])}
            live={index <= openThrough}
            tries={team.attempts?.[question.id] ?? 0}
            color={team.color}
          />
        ))}
      </div>

      <span className="hud-note">
        {solvedCount} of {questions.length} solved
        {totalTries > 0 && ` · ${totalTries} submission${totalTries > 1 ? "s" : ""}`}
      </span>
    </div>
  );
}
