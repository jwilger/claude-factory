# Semantic Types — Python

## NewType — zero-cost semantic distinction

`NewType` creates a distinct type at the type-checker level with zero runtime overhead:

```python
from typing import NewType

UserId = NewType('UserId', str)
ProductId = NewType('ProductId', str)
Quantity = NewType('Quantity', int)
EmailAddress = NewType('EmailAddress', str)

# mypy will catch transpositions at the call site
def send_welcome_email(to: EmailAddress) -> None: ...

user_id: UserId = UserId("user-123")
send_welcome_email(user_id)  # mypy error: Argument 1 to "send_welcome_email" has incompatible type "UserId"; expected "EmailAddress"
```

`NewType` only works at the type-checker level — at runtime `UserId("x")` is just `"x"`. For runtime validation, use Pydantic (below).

## Pydantic v2 — validated semantic types with runtime enforcement

```python
from pydantic import BaseModel, field_validator
from typing import Annotated
from pydantic.functional_validators import AfterValidator
import re

EMAIL_RE = re.compile(r'^[^@]+@[^@]+\.[^@]+$')

def validate_email(v: str) -> str:
    v = v.strip().lower()
    if not EMAIL_RE.match(v):
        raise ValueError(f'invalid email: {v!r}')
    return v

# Annotated types — parse once at the boundary, use everywhere after
EmailAddress = Annotated[str, AfterValidator(validate_email)]
PositiveInt = Annotated[int, AfterValidator(lambda v: v if v > 0 else (_ for _ in ()).throw(ValueError('must be positive')))]

class Order(BaseModel):
    model_config = {'frozen': True}  # immutable after construction

    id: str  # raw id from store is fine at the domain-model boundary
    customer_email: EmailAddress    # validated at construction
    quantity: PositiveInt           # validated at construction
```

## Combining NewType + Pydantic for full type safety

```python
from pydantic import RootModel
from typing import NewType

# NewType for mypy-level distinction
UserId = NewType('UserId', str)

# RootModel for runtime validation + mypy satisfaction
class UserIdModel(RootModel[str]):
    @classmethod
    def parse(cls, raw: str) -> 'UserIdModel':
        if not raw.strip():
            raise ValueError('UserId must not be empty')
        return cls(root=raw)
    
    def value(self) -> UserId:
        return UserId(self.root)

# Usage at I/O boundary
user_id = UserIdModel.parse(request_json['userId']).value()
# user_id is now UserId — mypy-checked and runtime-validated
```

## Dataclass-based value objects

For more complex domain types:

```python
from dataclasses import dataclass, field
from typing import ClassVar
import re

@dataclass(frozen=True)   # immutable — structural equality
class PhoneNumber:
    _PATTERN: ClassVar[re.Pattern] = re.compile(r'^\+?[\d\s\-()]{7,15}$')
    value: str

    def __post_init__(self) -> None:
        cleaned = self.value.replace(' ', '').replace('-', '').replace('(', '').replace(')', '')
        if not self._PATTERN.match(self.value) or len(cleaned) < 7:
            raise ValueError(f'invalid phone number: {self.value!r}')

    def __str__(self) -> str:
        return self.value
```

## The parse boundary

```python
# I/O boundary — JSON comes in, semantic types come out
def parse_create_order_command(body: dict) -> CreateOrderCommand:
    return CreateOrderCommand(
        customer_email=EmailAddress(body['email']),  # pydantic validates here
        quantity=PositiveInt(body['quantity']),       # pydantic validates here
    )

# Domain logic — only sees semantic types, never raw primitives
def apply_discount(email: EmailAddress, qty: Quantity) -> Money: ...
```
