//
// Created by fcors on 6/13/2026.
//

export module modulizer:options;

import :native;
import :core;
import std;

namespace modulizer {
export class OptionsBuilder;

struct OptionsDeleter {
  inline void operator()(Modulizer_Options *options) const noexcept {
    modulizer_options_destroy(options);
  }
};

using OptionsPtr = std::unique_ptr<Modulizer_Options, OptionsDeleter>;

export class Options {
  friend OptionsBuilder;

  constexpr explicit Options(Modulizer_Options *options) noexcept
      : options_{options} {}

public:
private:
  OptionsPtr options_;
};

struct BuilderDeleter {
  inline void operator()(Modulizer_Builder *builder) const noexcept {
    modulizer_builder_destroy(builder);
  }
};

using BuilderPtr = std::unique_ptr<Modulizer_Builder, BuilderDeleter>;

template <typename T>
concept ValidStringViewableRange =
    std::ranges::input_range<T> && std::ranges::viewable_range<T> &&
    !std::same_as<std::ranges::range_reference_t<T>, std::string &&> &&
    !std::same_as<std::ranges::range_reference_t<T>, const std::string &&>;

class OptionsBuilder {
public:
  inline OptionsBuilder() = default;

  template <NonConst Self>
  decltype(auto) name(this Self &&self, const std::string_view name) {
    modulizer_builder_set_name(self.builder.get(), to_c(name));
    return std::forward<Self>(self);
  }

  template <NonConst Self>
  decltype(auto) output_path(this Self &&self, const std::string_view path) {
    modulizer_builder_set_output_path(self.builder.get(), to_c(path));
    return std::forward<Self>(self);
  }

  template <NonConst Self>
  decltype(auto) library_header(this Self &&self, const IncludePath &path) {
    modulizer_builder_library_header(self.builder.get(), to_c(path));
    return std::forward<Self>(self);
  }

  template <NonConst Self, std::ranges::range Range>
    requires std::convertible_to<std::ranges::range_reference_t<Range>,
                                 const IncludePath &>
  decltype(auto) library_headers(this Self &&self, Range &&paths) {
    auto path_vector =
        std::forward<Range>(paths) |
        std::views::transform([](const auto &path) { return to_c(path); }) |
        std::ranges::to<std::vector>();
    modulizer_builder_library_headers(self.builder.get(), path_vector.data(),
                                      path_vector.size());
    return std::forward<Self>(self);
  }

  template <NonConst Self>
  decltype(auto) include_dir(this Self &&self, const std::string_view path) {
    modulizer_builder_include_dir(self.builder.get(), to_c(path));
    return std::forward<Self>(self);
  }

  template <NonConst Self, ValidStringViewableRange Range>
  decltype(auto) include_dirs(this Self &&self, Range &&paths) {
    auto path_vector =
        std::forward<Range>(paths) |
        std::views::transform([](const auto &path) { return to_c(path); }) |
        std::ranges::to<std::vector>();
    modulizer_builder_include_dirs(self.builder.get(), path_vector.data(),
                                   path_vector.size());
    return std::forward<Self>(self);
  }

  template <NonConst Self>
  decltype(auto) header_guard_format(this Self &&self,
                                     const std::string_view format) {
    modulizer_builder_set_header_guard_format(self.builder.get(), to_c(format));
    return std::forward<Self>(self);
  }

  template <NonConst Self>
  decltype(auto) expand_macro_from_definition(this Self &&self,
                                              const std::string_view name) {
    modulizer_builder_expand_macro_from_definition(self.builder.get(),
                                                   to_c(name));
    return std::forward<Self>(self);
  }

  template <NonConst Self, ValidStringViewableRange Range>
  decltype(auto) expand_macros_from_definition(this Self &&self,
                                               Range &&names) {
    auto name_vector =
        std::forward<Range>(names) |
        std::views::transform([](const auto &name) { return to_c(name); }) |
        std::ranges::to<std::vector>();
    modulizer_builder_expand_macros_from_definition(
        self.builder.get(), name_vector.data(), name_vector.size());
    return std::forward<Self>(self);
  }

  template <NonConst Self>
  decltype(auto) explicit_macro(this Self &&self, const std::string_view name) {
    modulizer_builder_explicit_macro(self.builder.get(), to_c(name));
    return std::forward<Self>(self);
  }

  template <NonConst Self, ValidStringViewableRange Range>
  decltype(auto) explicit_macros(this Self &&self, Range &&names) {
    auto definition_vector = std::forward<Range>(names) |
                             std::views::transform([](const auto &definition) {
                               return to_c(definition);
                             }) |
                             std::ranges::to<std::vector>();
    modulizer_builder_explicit_macros(
        self.builder.get(), definition_vector.data(), definition_vector.size());
    return std::forward<Self>(self);
  }

  template <NonConst Self>
  decltype(auto) exclude_symbol(this Self &&self, const std::string_view name) {
    modulizer_builder_exclude_symbol(self.builder.get(), to_c(name));
    return std::forward<Self>(self);
  }

  template <NonConst Self, ValidStringViewableRange Range>
  decltype(auto) exclude_symbols(this Self &&self, Range &&names) {
    auto name_vector =
        std::forward<Range>(names) |
        std::views::transform([](const auto &name) { return to_c(name); }) |
        std::ranges::to<std::vector>();
    modulizer_builder_exclude_symbols(self.builder.get(), name_vector.data(),
                                      name_vector.size());
    return std::forward<Self>(self);
  }

  template <NonConst Self>
  decltype(auto) include_symbol(this Self &&self, const std::string_view name) {
    modulizer_builder_include_symbol(self.builder.get(), to_c(name));
    return std::forward<Self>(self);
  }

  template <NonConst Self, ValidStringViewableRange Range>
  decltype(auto) include_symbols(this Self &&self, Range &&names) {
    auto name_vector =
        std::forward<Range>(names) |
        std::views::transform([](const auto &name) { return to_c(name); }) |
        std::ranges::to<std::vector>();
    modulizer_builder_include_symbols(self.builder.get(), name_vector.data(),
                                      name_vector.size());
    return std::forward<Self>(self);
  }

  [[nodiscard]] inline Options build() const {
    return Options{modulizer_options_create(builder.get())};
  }

private:
  BuilderPtr builder{modulizer_builder_create()};
};
} // namespace modulizer
