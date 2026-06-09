//====================================================
// Namespaces
//====================================================

namespace Engine {}

namespace Engine::Core {}

namespace EC = Engine::Core;

using namespace Engine;
using Engine::Core;

//====================================================
// Typedefs / Aliases
//====================================================

typedef int Int32;
typedef const char* CString;

typedef void (*Callback)(int);
typedef int (*MathFn)(int, int);

using String = std::string;
using IntList = std::vector<int>;

using Predicate = bool(*)(int);

//====================================================
// Enums
//====================================================

enum Color
{
    Red,
    Green,
    Blue
};

enum class Direction
{
    North,
    South,
    East,
    West
};

enum class Byte : unsigned char
{
    A,
    B
};

//====================================================
// Structs / Classes / Unions
//====================================================

struct Empty {};

struct Base
{
    int Value;
};

struct Derived final : Base
{
};

class Actor
{
public:
    void Tick();
};

union Variant
{
    int IntValue;
    float FloatValue;
};

//====================================================
// Variables
//====================================================

int GlobalInt;

extern int ExternalInt;

inline constexpr int CompileTimeValue = 42;

const char* GlobalString;

//====================================================
// Arrays
//====================================================

int FixedArray[10];

extern float LookupTable[256];

//====================================================
// References / Pointers
//====================================================

int* Ptr;

const int* ConstPtr;

int& Ref = GlobalInt;

int&& RValueRef = 42;

//====================================================
// Functions
//====================================================

void SimpleFunction();

int Add(int a, int b);

const char* GetName();

void NoexceptFunction() noexcept;

[[nodiscard]]
int Compute();

//====================================================
// Function Pointer Variables
//====================================================

void (*GlobalCallback)(int);

int (*GlobalMathFn)(int, int);

void (*SignalHandlers[8])(int);

//====================================================
// Function Returning Function Pointer
//====================================================

int (*GetMathFunction())(int, int);

//====================================================
// References To Function Pointers
//====================================================

void (&CallbackRef)(int) = *GlobalCallback;

//====================================================
// Member Pointers
//====================================================

struct MemberExample
{
    int Value;

    void Method();
};

int MemberExample::*ValuePtr;

void (MemberExample::*MethodPtr)();

//====================================================
// Templates
//====================================================

template<typename T>
struct Vector
{
};

template<typename T, typename U>
class Pair
{
};

template<typename T>
using Ptr = T*;

//====================================================
// Concepts
//====================================================

template<typename T>
concept Incrementable = requires(T t)
{
    ++t;
};

//====================================================
// Variable Templates
//====================================================

template<typename T>
inline constexpr bool IsVoid = false;

//====================================================
// Function Templates
//====================================================

template<typename T>
T Max(T a, T b);

//====================================================
// Friend Declarations
//====================================================

class FriendOwner
{
    friend class FriendClass;

    friend void FriendFunction();
};

//====================================================
// Nested Types
//====================================================

class Outer
{
public:
    struct Inner
    {
        enum class State
        {
            Idle,
            Running
        };
    };
};

//====================================================
// Attributes
//====================================================

[[nodiscard]]
int ImportantFunction();

[[deprecated]]
void OldFunction();

//====================================================
// Trailing Return Types
//====================================================

auto ModernFunction() -> int;

auto Factory() -> Actor*;

//====================================================
// Auto Variables
//====================================================

auto AutoValue = 42;

//====================================================
// Inline Namespaces
//====================================================

inline namespace V1
{
    struct VersionedType {};
}