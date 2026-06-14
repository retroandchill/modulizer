//
// Created by fcors on 6/14/2026.
//

export module modulizer:generation_result;

import std;
import :native;
import :core;

namespace modulizer
{
    struct GenerationResultDeleter
    {
        inline void operator()(Modulizer_GenerationResult *result) const noexcept
        {
            modulizer_generation_result_destroy(result);
        }
    };

    using GenerationResultPtr = std::unique_ptr<Modulizer_GenerationResult, GenerationResultDeleter>;

    export class GenerationResult
    {
        friend class Builder;

        constexpr explicit GenerationResult(Modulizer_GenerationResult *ptr) : ptr_{ptr}
        {
        }

      public:
        [[nodiscard]] inline std::string_view output_path() const noexcept
        {
            return from_c(modulizer_generation_result_get_output_path(ptr_.get()));
        }

      private:
        GenerationResultPtr ptr_;
    };
} // namespace modulizer
