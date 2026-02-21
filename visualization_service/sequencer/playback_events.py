from __future__ import annotations

from typing import Literal

from pydantic import BaseModel


class StepForward(BaseModel):
    type: Literal["step_forward"] = "step_forward"


class StepBack(BaseModel):
    type: Literal["step_back"] = "step_back"


class JumpTo(BaseModel):
    type: Literal["jump"] = "jump"
    step: int


class Reset(BaseModel):
    type: Literal["reset"] = "reset"


class SetSpeed(BaseModel):
    type: Literal["set_speed"] = "set_speed"
    multiplier: float


class SetPlaying(BaseModel):
    type: Literal["set_playing"] = "set_playing"
    value: bool


PlaybackEvent = StepForward | StepBack | JumpTo | Reset | SetSpeed | SetPlaying
