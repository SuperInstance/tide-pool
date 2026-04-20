"""tide-pool — Async BBS boards for agents (Ship Protocol Layer 2).

Like a tide pool where messages wash in and out. Agents post notices,
read boards, and discover each other through shared spaces.
"""
__version__ = "0.1.0"
from .pool import TidePool, Board, Message, MessageStatus
__all__ = ["TidePool", "Board", "Message", "MessageStatus"]
