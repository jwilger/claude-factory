# Railway-Oriented Programming — Python

## returns library

```bash
pip install returns
```

```python
from returns.result import Result, Success, Failure
from returns.pipeline import flow, pipe
from returns.pointfree import bind, map_

# Domain error types — plain dataclasses or named tuples
from dataclasses import dataclass

@dataclass(frozen=True)
class EmailTaken:
    email: EmailAddress

@dataclass(frozen=True)
class WeakPassword:
    reason: str

@dataclass(frozen=True)  
class InvalidEmail:
    value: str

RegistrationError = EmailTaken | WeakPassword | InvalidEmail

# Fallible function — returns Result, never raises for expected failures
def register_user(email: EmailAddress, password: str) -> Result['UserId', RegistrationError]:
    if not is_strong_password(password):
        return Failure(WeakPassword(reason='must be at least 12 chars'))
    user_id = generate_user_id()
    return Success(user_id)
```

## Chaining with bind and map

```python
from returns.result import Result
from returns.pipeline import pipe
from returns.pointfree import bind, map_

def process_registration(
    raw_email: str,
    raw_password: str,
) -> Result[RegistrationConfirmation, RegistrationError | EmailParseError]:
    return pipe(
        parse_email(raw_email),              # Result[EmailAddress, EmailParseError]
        bind(check_availability),            # Result[EmailAddress, RegistrationError]
        bind(lambda email: register_user(email, raw_password)),  # Result[UserId, RegistrationError]
        map_(build_confirmation),            # Result[RegistrationConfirmation, ...]
    )
```

## Hand-rolled Result (no library dependency)

If `returns` is too heavy or introduces dependency concerns:

```python
from __future__ import annotations
from typing import TypeVar, Generic, Callable, NoReturn
from dataclasses import dataclass

T = TypeVar('T')
E = TypeVar('E')
U = TypeVar('U')
F = TypeVar('F')

@dataclass(frozen=True)
class Ok(Generic[T]):
    value: T

    def map(self, f: Callable[[T], U]) -> 'Ok[U]':
        return Ok(f(self.value))

    def bind(self, f: Callable[[T], 'Result[U, E]']) -> 'Result[U, E]':
        return f(self.value)

    def unwrap(self) -> T:
        return self.value

@dataclass(frozen=True)
class Err(Generic[E]):
    error: E

    def map(self, f: object) -> 'Err[E]':
        return self

    def bind(self, f: object) -> 'Err[E]':
        return self

    def unwrap(self) -> NoReturn:
        raise ValueError(f'Called unwrap on Err: {self.error}')

Result = Ok[T] | Err[E]
```

## Pattern matching at the boundary (Python 3.10+)

```python
# At the application boundary, handle all error cases explicitly
result = process_registration(email_str, password)

match result:
    case Ok(value=confirmation):
        return jsonify({'confirmationId': str(confirmation.id)}), 201
    case Err(error=EmailTaken(email=e)):
        return jsonify({'error': f'Email {e} already registered'}), 409
    case Err(error=WeakPassword(reason=r)):
        return jsonify({'error': r}), 422
    case Err(error=InvalidEmail(value=v)):
        return jsonify({'error': f'Invalid email: {v}'}), 422
    # mypy exhaustiveness: add `_: Never` arm to catch future variants
```

## The rule against exceptions for control flow

```python
# Wrong — exception for an expected outcome
def find_order(order_id: OrderId) -> Order:
    order = db.query(Order).filter_by(id=order_id).first()
    if order is None:
        raise OrderNotFoundError(order_id)  # caller must remember try/except
    return order

# Right — return type makes the failure visible
def find_order(order_id: OrderId) -> Result[Order, OrderNotFound]:
    order = db.query(Order).filter_by(id=order_id).first()
    if order is None:
        return Err(OrderNotFound(order_id))
    return Ok(order)
```

Exceptions remain appropriate for: programmer errors (assertion failures, type contract violations), truly unrecoverable system failures (OOM, corrupted state).
