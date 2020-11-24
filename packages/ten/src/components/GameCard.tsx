import React, { ReactElement } from 'react';

import { Board, getGameStatus } from '../game/board';
import type { GameState } from '../game/game-state';
import BoardGrid from './BoardGrid';
import styles from './GameCard.module.css';

const getMessage = (
  board: Board,
  playerCanMove: boolean,
  playerMadeIllegalMove: boolean
): string => {
  switch (getGameStatus(board)) {
    case 0:
      if (!playerCanMove) {
        return 'Waiting for AI move.';
      }
      return playerMadeIllegalMove ? 'Illegal move!' : 'Waiting for your move.';
    case 1:
      return 'Black Wins';
    case -1:
      return 'White Wins';
  }
};

type Props = {
  readonly gameState: GameState;
  readonly playerCanMove: boolean;
  readonly playerMadeIllegalMove: boolean;
  readonly showGameStarterButtons: boolean;
  readonly showUndoButton: boolean;
  readonly clickCallback: (a: number, b: number) => void;
  readonly onSelectSide: (side: 1 | -1) => void;
  readonly onUndoMove: () => void;
};

export default function GameCard({
  gameState: { board, highlightedCell, aiInfo },
  playerCanMove,
  playerMadeIllegalMove,
  showGameStarterButtons,
  showUndoButton,
  clickCallback,
  onSelectSide,
  onUndoMove,
}: Props): ReactElement {
  const { tiles, playerIdentity } = board;
  let aiInfoNode: ReactElement | null;
  if (aiInfo == null) {
    aiInfoNode = null;
  } else {
    const { winningPercentage, simulationCounter } = aiInfo;
    aiInfoNode = (
      <div className="card__body">
        {`AI Winning Probability ${winningPercentage}%. Number of Simulations Run: ${simulationCounter}.`}
      </div>
    );
  }
  return (
    <div className="card">
      <div className="card__body">{getMessage(board, playerCanMove, playerMadeIllegalMove)}</div>
      <div className="card__body">
        {`Your Identity: ${playerIdentity === 1 ? 'Black' : 'White'}`}
      </div>
      {aiInfoNode}
      <div className={`card__body ${styles.GameCells}`}>
        {!playerCanMove && <div className={styles.Overlay} />}
        <BoardGrid tiles={tiles} highlightedCell={highlightedCell} clickCallback={clickCallback} />
      </div>
      {showGameStarterButtons && (
        <div className="card__footer">
          <button
            className="button button--outline button--primary"
            onClick={() => onSelectSide(1)}
          >
            Play as Black
          </button>
          <button
            className="button button--outline button--primary"
            onClick={() => onSelectSide(-1)}
          >
            Play as White
          </button>
        </div>
      )}
      {showUndoButton && (
        <div className="card__footer">
          <button className="button button--outline button--primary" onClick={onUndoMove}>
            Undo your last move
          </button>
        </div>
      )}
    </div>
  );
}
