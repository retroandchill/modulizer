//
// Created by fcors on 6/13/2026.
//

export module modulizer:core;

import std;
import :native;

namespace modulizer {
template <typename T>
concept NonConst = !std::is_const_v<T>;

export class Error : public std::runtime_error {
  using std::runtime_error::runtime_error;
};

constexpr void throw_if_invalid(const bool result) {
  if (!result) {
    throw Error{modulizer_get_last_error()};
  }
}

template <typename T> constexpr void throw_if_invalid(T *) = delete;

template <typename... Functors> struct Overload : Functors... {
  using Functors::operator()...;
};

template <typename... Functors> Overload(Functors...) -> Overload<Functors...>;

constexpr Modulizer_StringView to_c(std::string_view str) {
  return {str.data(), str.size()};
}

export struct IfdefInclude {
  std::string_view path;
  std::string_view if_defined;
};

export struct ConditionalInclude {
  std::string_view path;
  std::string_view condition;
};

export using IncludePath =
    std::variant<std::string_view, IfdefInclude, ConditionalInclude>;

constexpr Modulizer_IncludePath to_c(const IncludePath &path) {
  return std::visit(
      Overload{[](std::string_view path) {
                 return Modulizer_IncludePath{
                     .kind = MODULIZER_INCLUDE_TYPE_UNCONDITIONAL,
                     .unconditional = {.path = to_c(path)}};
               },
               [](const IfdefInclude &include) {
                 return Modulizer_IncludePath{
                     .kind = MODULIZER_INCLUDE_TYPE_IFDEF,
                     .ifdef = {.path = to_c(include.path),
                               .if_defined = to_c(include.if_defined)}};
               },
               [](const ConditionalInclude &include) {
                 return Modulizer_IncludePath{
                     .kind = MODULIZER_INCLUDE_TYPE_CONDITIONAL,
                     .conditional = {.path = to_c(include.path),
                                     .condition = to_c(include.condition)}};
               }},
      path);
}
} // namespace modulizer
