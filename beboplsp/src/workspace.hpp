#pragma once

#include <bebop.h>
#include <lsp/types.h>
#include <string>
#include <unordered_map>
#include <vector>

namespace beboplsp {

struct DiagnosticInfo {
    lsp::Range range;
    std::string hint;
    std::string suggestion;
};

struct OpenDocument {
    lsp::DocumentUri uri;
    std::string content;
    std::vector<size_t> lineOffsets;
    std::vector<lsp::Diagnostic> diagnostics;
    std::vector<DiagnosticInfo> diagnosticInfos;

    bebop_context_t* ctx = nullptr;
    bebop_parse_result_t* result = nullptr;

    OpenDocument() = default;
    ~OpenDocument() {
        if (ctx) {
            bebop_context_destroy(ctx);
        }
    }

    OpenDocument(const OpenDocument&) = delete;
    OpenDocument& operator=(const OpenDocument&) = delete;

    OpenDocument(OpenDocument&& other) noexcept
        : uri(std::move(other.uri))
        , content(std::move(other.content))
        , lineOffsets(std::move(other.lineOffsets))
        , diagnostics(std::move(other.diagnostics))
        , diagnosticInfos(std::move(other.diagnosticInfos))
        , ctx(other.ctx)
        , result(other.result)
    {
        other.ctx = nullptr;
        other.result = nullptr;
    }

    OpenDocument& operator=(OpenDocument&& other) noexcept {
        if (this != &other) {
            if (ctx) {
                bebop_context_destroy(ctx);
            }
            uri = std::move(other.uri);
            content = std::move(other.content);
            lineOffsets = std::move(other.lineOffsets);
            diagnostics = std::move(other.diagnostics);
            diagnosticInfos = std::move(other.diagnosticInfos);
            ctx = other.ctx;
            result = other.result;
            other.ctx = nullptr;
            other.result = nullptr;
        }
        return *this;
    }

    void buildLineIndex();
    [[nodiscard]] lsp::Position offsetToPosition(uint32_t offset) const;
    [[nodiscard]] uint32_t positionToOffset(lsp::Position pos) const;
    [[nodiscard]] lsp::Range spanToRange(bebop_span_t span) const;
    [[nodiscard]] bool valid() const { return result != nullptr; }
};

class Workspace {
public:
    explicit Workspace(std::vector<std::string> includePaths);
    ~Workspace();

    Workspace(const Workspace&) = delete;
    Workspace& operator=(const Workspace&) = delete;

    void addIncludePath(const std::string& path);

    void openDocument(const lsp::DocumentUri& uri, std::string content);
    void updateDocument(const lsp::DocumentUri& uri, const std::string& content);
    void closeDocument(const lsp::DocumentUri& uri);

    [[nodiscard]] OpenDocument* getDocument(const std::string& path);
    [[nodiscard]] const OpenDocument* getDocument(const std::string& path) const;

    template<typename Func>
    void forEachDocument(Func&& func) {
        for (auto& [path, doc] : openDocuments_) {
            func(path, doc);
        }
    }

    [[nodiscard]] const std::vector<std::string>& includePaths() const { return includePaths_; }
    [[nodiscard]] std::string uriToPath(const lsp::DocumentUri& uri) const;

    // Type index: maps type FQN or simple name -> import path (relative to include path)
    [[nodiscard]] const std::string* findImportForType(const std::string& typeName) const;
    void rebuildTypeIndex();

private:
    void parseDocument(OpenDocument& doc);
    void collectDiagnostics(OpenDocument& doc);
    void scanIncludePathsForTypes();

    std::vector<std::string> includePaths_;
    std::unordered_map<std::string, OpenDocument> openDocuments_;
    std::unordered_map<std::string, std::string> typeIndex_; // FQN/name -> import path

    std::vector<const char*> includeCStrs_;
    bool typeIndexBuilt_ = false;
};

} // namespace beboplsp
