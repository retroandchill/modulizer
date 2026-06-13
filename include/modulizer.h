//
// Created by fcors on 6/13/2026.
//
#pragma once

#ifndef MODULIZER_EXPORT_MODULE
#include <string.h>
#endif

#ifdef MODULIZER_SHARED
#ifdef _WIN32
#ifdef MODULIZER_EXPORT
#define MODULIZER_API __declspec(dllexport)
#else
#define MODULIZER_API __declspec(dllimport)
#endif
#else
#define MODULIZER_API = __attribute__((visibility("default")))
#endif
#else
#define MODULIZER_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Modulizer_Builder Modulizer_Builder;
typedef struct Modulizer_Options Modulizer_Options;

typedef struct Modulizer_StringView {
  const char *data;
  size_t length;
} Modulizer_StringView;

inline Modulizer_StringView modulizer_string_view(const char *data) {
  return {data, strlen(data)};
}

typedef enum Modulizer_IncludeType {
  MODULIZER_INCLUDE_TYPE_UNCONDITIONAL = 0,
  MODULIZER_INCLUDE_TYPE_IFDEF = 1,
  MODULIZER_INCLUDE_TYPE_CONDITIONAL = 2,
} Modulizer_IncludeType;

typedef struct Modulizer_IncludePath {
  Modulizer_IncludeType kind;
  union {
    struct {
      Modulizer_StringView path;
    } unconditional;
    struct {
      Modulizer_StringView path;
      Modulizer_StringView if_defined;
    } ifdef;
    struct {
      Modulizer_StringView path;
      Modulizer_StringView condition;
    } conditional;
  };
} Modulizer_IncludePath;

extern MODULIZER_API const char *modulizer_get_last_error(void);

extern MODULIZER_API Modulizer_Builder *modulizer_builder_create(void);

extern MODULIZER_API void modulizer_builder_destroy(Modulizer_Builder *builder);

extern MODULIZER_API bool modulizer_builder_set_name(Modulizer_Builder *builder,
                                                     Modulizer_StringView name);

extern MODULIZER_API bool
modulizer_builder_set_output_path(Modulizer_Builder *builder,
                                  Modulizer_StringView path);

extern MODULIZER_API bool
modulizer_builder_library_header(Modulizer_Builder *builder,
                                 const Modulizer_IncludePath *path);

extern MODULIZER_API bool
modulizer_builder_library_headers(Modulizer_Builder *builder,
                                  const Modulizer_IncludePath *paths,
                                  size_t count);

extern MODULIZER_API bool
modulizer_builder_include_dir(Modulizer_Builder *builder,
                              Modulizer_StringView path);

extern MODULIZER_API bool
modulizer_builder_include_dirs(Modulizer_Builder *builder,
                               const Modulizer_StringView *paths, size_t count);

extern MODULIZER_API bool
modulizer_builder_set_header_guard_format(Modulizer_Builder *builder,
                                          Modulizer_StringView format);

extern MODULIZER_API bool
modulizer_builder_expand_macro_from_definition(Modulizer_Builder *builder,
                                               Modulizer_StringView macro_name);

extern MODULIZER_API bool modulizer_builder_expand_macros_from_definition(
    Modulizer_Builder *builder, const Modulizer_StringView *names,
    size_t count);

extern MODULIZER_API bool
modulizer_builder_explicit_macro(Modulizer_Builder *builder,
                                 Modulizer_StringView definition);

extern MODULIZER_API bool
modulizer_builder_explicit_macros(Modulizer_Builder *builder,
                                  const Modulizer_StringView *definitions,
                                  size_t count);

extern MODULIZER_API bool
modulizer_builder_implementation_macro(Modulizer_Builder *builder,
                                       Modulizer_StringView name);

extern MODULIZER_API bool
modulizer_builder_implementation_macros(Modulizer_Builder *builder,
                                        const Modulizer_StringView *names,
                                        size_t count);

extern MODULIZER_API bool
modulizer_builder_exclude_symbol(Modulizer_Builder *builder,
                                 Modulizer_StringView name);

extern MODULIZER_API bool
modulizer_builder_exclude_symbols(Modulizer_Builder *builder,
                                  const Modulizer_StringView *names,
                                  size_t count);

extern MODULIZER_API bool
modulizer_builder_include_symbol(Modulizer_Builder *builder,
                                 Modulizer_StringView name);

extern MODULIZER_API void
modulizer_builder_include_symbols(Modulizer_Builder *builder,
                                  const Modulizer_StringView *names,
                                  size_t count);

extern MODULIZER_API Modulizer_Options *
modulizer_options_create(const Modulizer_Builder *builder);

extern MODULIZER_API void modulizer_options_destroy(Modulizer_Options *options);

#ifdef __cplusplus
}
#endif
