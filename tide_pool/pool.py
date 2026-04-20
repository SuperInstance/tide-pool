"""Tide pool: async message boards for agent communication."""

import time
import hashlib
from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional, Set


class MessageStatus(Enum):
    FRESH = "fresh"
    READ = "read"
    EXPIRED = "expired"
    PINNED = "pinned"


@dataclass
class Message:
    """A message washed up on the tide pool shore."""
    id: str
    author: str
    board: str
    content: str
    timestamp: float = field(default_factory=time.time)
    ttl: float = 86400.0  # time-to-live in seconds (default 24h)
    status: MessageStatus = MessageStatus.FRESH
    tags: List[str] = field(default_factory=list)
    
    @staticmethod
    def make_id(author: str, board: str, content: str, ts: float) -> str:
        raw = f"{author}:{board}:{content[:64]}:{ts}"
        return hashlib.sha256(raw.encode()).hexdigest()[:12]
    
    def is_expired(self, now: Optional[float] = None) -> bool:
        if self.status == MessageStatus.PINNED:
            return False
        now = now or time.time()
        return (now - self.timestamp) > self.ttl


@dataclass
class Board:
    """A message board in the tide pool."""
    name: str
    description: str = ""
    max_messages: int = 1000
    subscribers: Set[str] = field(default_factory=set)
    _messages: Dict[str, Message] = field(default_factory=dict, repr=False)
    
    def post(self, msg: Message) -> bool:
        if len(self._messages) >= self.max_messages:
            self._evict_expired()
        if len(self._messages) >= self.max_messages:
            return False  # board full
        self._messages[msg.id] = msg
        return True
    
    def read(self, since: float = 0.0, tags: Optional[List[str]] = None) -> List[Message]:
        msgs = [m for m in self._messages.values() 
                if m.timestamp > since and not m.is_expired()]
        if tags:
            msgs = [m for m in msgs if any(t in m.tags for t in tags)]
        msgs.sort(key=lambda m: m.timestamp, reverse=True)
        for m in msgs:
            if m.status == MessageStatus.FRESH:
                m.status = MessageStatus.READ
        return msgs
    
    def _evict_expired(self) -> int:
        now = time.time()
        expired = [mid for mid, m in self._messages.items() if m.is_expired(now)]
        for mid in expired:
            del self._messages[mid]
        return len(expired)
    
    def subscribe(self, agent_id: str) -> None:
        self.subscribers.add(agent_id)
    
    def unsubscribe(self, agent_id: str) -> None:
        self.subscribers.discard(agent_id)
    
    def message_count(self) -> int:
        return len(self._messages)


class TidePool:
    """Collection of message boards. Ship Protocol Layer 2.
    
    Usage:
        pool = TidePool()
        pool.create_board("fleet-ops", "Fleet operations notices")
        pool.post("fleet-ops", Message(id="x", author="oracle1", 
                  board="fleet-ops", content="Crate published!"))
        msgs = pool.read("fleet-ops", since=time.time() - 3600)
    """
    
    def __init__(self):
        self.boards: Dict[str, Board] = {}
    
    def create_board(self, name: str, description: str = "", 
                     max_messages: int = 1000) -> Board:
        board = Board(name=name, description=description, max_messages=max_messages)
        self.boards[name] = board
        return board
    
    def post(self, board_name: str, content: str, author: str,
             ttl: float = 86400.0, tags: Optional[List[str]] = None) -> Optional[Message]:
        if board_name not in self.boards:
            return None
        ts = time.time()
        msg = Message(
            id=Message.make_id(author, board_name, content, ts),
            author=author, board=board_name, content=content,
            timestamp=ts, ttl=ttl, tags=tags or [],
        )
        if self.boards[board_name].post(msg):
            return msg
        return None
    
    def read(self, board_name: str, since: float = 0.0,
             tags: Optional[List[str]] = None) -> List[Message]:
        if board_name not in self.boards:
            return []
        return self.boards[board_name].read(since=since, tags=tags)
    
    def subscribe(self, board_name: str, agent_id: str) -> bool:
        if board_name not in self.boards:
            return False
        self.boards[board_name].subscribe(agent_id)
        return True
    
    def boards_for_agent(self, agent_id: str) -> List[str]:
        return [name for name, board in self.boards.items() 
                if agent_id in board.subscribers]
    
    def stats(self) -> dict:
        return {
            "boards": len(self.boards),
            "total_messages": sum(b.message_count() for b in self.boards.values()),
            "total_subscribers": sum(len(b.subscribers) for b in self.boards.values()),
        }
