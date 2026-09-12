function solvedPrefix(team, questions) {
  const at = questions.findIndex((question) => !team?.solves?.[question.id]);
  return at === -1 ? questions.length : at;
}

function Dots({ team, questions }) {
  const cleared = solvedPrefix(team, questions);

  return (
    <div className="scorebug-dots">
      {questions.map((question, index) => {
        const solved = Boolean(team.solves?.[question.id]);
        const state = solved ? "past" : index === cleared ? "live" : "next";
        return (
          <span
            key={question.id}
            className={`dot dot--${state}`}
            title={`Q${index + 1}: ${question.title}`}
          />
        );
      })}
    </div>
  );
}

function Side({ team, side, leading, questions }) {
  const classes = ["scorebug-side", `scorebug-side--${side}`];
  if (leading) classes.push("scorebug-side--leading");

  return (
    <div className={classes.join(" ")} style={{ "--team": team.color } as React.CSSProperties}>
      <div className="scorebug-plate">
        <span className="scorebug-team">{team.name}</span>
        <Dots team={team} questions={questions} />
      </div>
      <span className="scorebug-score" key={team.score}>
        {team.score}
      </span>
    </div>
  );
}

export default function Scoreboard({ left, right, questions, leader }) {
  if (!left || !right) return null;

  return (
    <div className="scorebug">
      <Side team={left} side="left" leading={leader === "left"} questions={questions} />

      <div className="scorebug-centre">
        <span className="scorebug-versus">VS</span>
      </div>

      <Side team={right} side="right" leading={leader === "right"} questions={questions} />
    </div>
  );
}
