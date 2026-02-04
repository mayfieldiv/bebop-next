#include "workspace.hpp"
#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <regex>

namespace fs = std::filesystem;

namespace beboplsp {

namespace {

void* hostAlloc(void* ptr, size_t oldSize, size_t newSize, void* ctx)
{
    (void)oldSize;
    (void)ctx;
    if (newSize == 0) {
        std::free(ptr);
        return nullptr;
    }
    return std::realloc(ptr, newSize);
}

bebop_file_result_t hostFileRead(const char* path, void* ctx)
{
    (void)ctx;
    std::ifstream file(path, std::ios::binary | std::ios::ate);
    if (!file) {
        return {nullptr, 0, "File not found"};
    }
    auto size = file.tellg();
    file.seekg(0, std::ios::beg);
    char* content = static_cast<char*>(std::malloc(static_cast<size_t>(size) + 1));
    if (!content) {
        return {nullptr, 0, "Out of memory"};
    }
    file.read(content, size);
    content[size] = '\0';
    return {content, static_cast<size_t>(size), nullptr};
}

bool hostFileExists(const char* path, void* ctx)
{
    (void)ctx;
    return fs::exists(path) && fs::is_regular_file(path);
}

std::string extractSuggestion(const char* text)
{
    if (!text) return {};
    std::regex re(R"(did you mean ['\"]([^'\"]+)['\"])");
    std::smatch match;
    std::string s(text);
    if (std::regex_search(s, match, re) && match.size() > 1) {
        return match[1].str();
    }
    return {};
}

} // namespace

void OpenDocument::buildLineIndex()
{
    lineOffsets.clear();
    lineOffsets.push_back(0);
    for (size_t i = 0; i < content.size(); ++i) {
        if (content[i] == '\n') {
            lineOffsets.push_back(i + 1);
        } else if (content[i] == '\r') {
            // Handle \r\n and standalone \r
            if (i + 1 >= content.size() || content[i + 1] != '\n') {
                lineOffsets.push_back(i + 1);
            }
        }
    }
}

lsp::Position OpenDocument::offsetToPosition(uint32_t offset) const
{
    if (lineOffsets.empty()) {
        return {0, offset};
    }
    auto it = std::upper_bound(lineOffsets.begin(), lineOffsets.end(), offset);
    if (it == lineOffsets.begin()) {
        return {0, offset};
    }
    --it;
    auto line = static_cast<lsp::uint>(std::distance(lineOffsets.begin(), it));
    auto col = static_cast<lsp::uint>(offset - *it);
    return {line, col};
}

uint32_t OpenDocument::positionToOffset(lsp::Position pos) const
{
    if (pos.line >= lineOffsets.size()) {
        return static_cast<uint32_t>(content.size());
    }
    return static_cast<uint32_t>(lineOffsets[pos.line] + pos.character);
}

lsp::Range OpenDocument::spanToRange(bebop_span_t span) const
{
    // Bebop uses 1-based line/col; LSP uses 0-based
    return {
        offsetToPosition(span.off),
        offsetToPosition(span.off + span.len)
    };
}

Workspace::Workspace(std::vector<std::string> includePaths)
    : includePaths_(std::move(includePaths))
{
}

Workspace::~Workspace()
{
    for (auto& [path, doc] : openDocuments_) {
        if (doc.ctx) {
            bebop_context_destroy(doc.ctx);
        }
    }
}

void Workspace::addIncludePath(const std::string& path)
{
    if (std::find(includePaths_.begin(), includePaths_.end(), path) == includePaths_.end()) {
        includePaths_.push_back(path);
        typeIndexBuilt_ = false; // Invalidate index
    }
}

std::string Workspace::uriToPath(const lsp::DocumentUri& uri) const
{
    // Use full URI string as key for untitled documents
    if (uri.scheme() == "untitled") {
        return uri.toString();
    }
    return std::string(uri.path());
}

void Workspace::openDocument(const lsp::DocumentUri& uri, std::string content)
{
    std::string path = uriToPath(uri);

    // Only add include path for real files
    if (uri.scheme() != "untitled") {
        fs::path filePath(path);
        if (filePath.has_parent_path()) {
            addIncludePath(filePath.parent_path().string());
        }
    }

    OpenDocument doc;
    doc.uri = uri;
    doc.content = std::move(content);
    doc.buildLineIndex();

    parseDocument(doc);
    openDocuments_[path] = std::move(doc);
}

void Workspace::updateDocument(const lsp::DocumentUri& uri, const std::string& content)
{
    std::string path = uriToPath(uri);
    auto it = openDocuments_.find(path);
    if (it != openDocuments_.end()) {
        if (it->second.ctx) {
            bebop_context_destroy(it->second.ctx);
            it->second.ctx = nullptr;
            it->second.result = nullptr;
        }

        it->second.content = content;
        it->second.buildLineIndex();
        parseDocument(it->second);
    }
}

void Workspace::closeDocument(const lsp::DocumentUri& uri)
{
    std::string path = uriToPath(uri);
    auto it = openDocuments_.find(path);
    if (it != openDocuments_.end()) {
        if (it->second.ctx) {
            bebop_context_destroy(it->second.ctx);
        }
        openDocuments_.erase(it);
    }
}

void Workspace::parseDocument(OpenDocument& doc)
{
    doc.diagnostics.clear();
    doc.diagnosticInfos.clear();
    doc.result = nullptr;

    if (doc.ctx) {
        bebop_context_destroy(doc.ctx);
        doc.ctx = nullptr;
    }

    includeCStrs_.clear();
    for (const auto& p : includePaths_) {
        includeCStrs_.push_back(p.c_str());
    }

    bebop_includes_t includes = {
        .paths = includeCStrs_.empty() ? nullptr : includeCStrs_.data(),
        .count = static_cast<uint32_t>(includeCStrs_.size())
    };

    bebop_host_t host = {};
    host.allocator.alloc = hostAlloc;
    host.file_reader.read = hostFileRead;
    host.file_reader.exists = hostFileExists;
    host.includes = includes;

    doc.ctx = bebop_context_create(&host);
    if (!doc.ctx) {
        doc.diagnostics.push_back({
            .range = {{0, 0}, {0, 0}},
            .message = "Failed to create bebop context",
            .severity = lsp::DiagnosticSeverity::Error,
            .source = "bebop"
        });
        return;
    }

    std::string path(doc.uri.path());
    bebop_source_t source = {
        .source = doc.content.c_str(),
        .len = doc.content.size(),
        .path = path.c_str()
    };

    bebop_status_t status = bebop_parse_source(doc.ctx, &source, &doc.result);

    if (status == BEBOP_FATAL) {
        doc.diagnostics.push_back({
            .range = {{0, 0}, {0, 0}},
            .message = bebop_context_error_message(doc.ctx)
                ? bebop_context_error_message(doc.ctx)
                : "Fatal parse error",
            .severity = lsp::DiagnosticSeverity::Error,
            .source = "bebop"
        });
        return;
    }

    collectDiagnostics(doc);
}

void Workspace::collectDiagnostics(OpenDocument& doc)
{
    if (!doc.result) {
        return;
    }

    uint32_t count = bebop_result_diagnostic_count(doc.result);
    for (uint32_t i = 0; i < count; ++i) {
        const bebop_diagnostic_t* diag = bebop_result_diagnostic_at(doc.result, i);
        if (!diag) {
            continue;
        }

        lsp::Diagnostic lspDiag;

        switch (bebop_diagnostic_severity(diag)) {
            case BEBOP_DIAG_ERROR:
                lspDiag.severity = lsp::DiagnosticSeverity::Error;
                break;
            case BEBOP_DIAG_WARNING:
                lspDiag.severity = lsp::DiagnosticSeverity::Warning;
                break;
            case BEBOP_DIAG_INFO:
                lspDiag.severity = lsp::DiagnosticSeverity::Information;
                break;
        }

        if (const char* msg = bebop_diagnostic_message(diag)) {
            lspDiag.message = msg;
        }

        lspDiag.range = doc.spanToRange(bebop_diagnostic_span(diag));

        std::vector<lsp::DiagnosticRelatedInformation> relatedInfo;

        if (const char* hint = bebop_diagnostic_hint(diag)) {
            if (hint[0] != '\0') {  // Skip empty hints
                relatedInfo.push_back({
                    .location = {.uri = doc.uri, .range = lspDiag.range},
                    .message = hint
                });
            }
        }

        uint32_t labelCount = bebop_diagnostic_label_count(diag);
        for (uint32_t j = 0; j < labelCount; ++j) {
            const char* labelMsg = bebop_diagnostic_label_message(diag, j);
            if (!labelMsg || labelMsg[0] == '\0') {  // Skip empty labels
                continue;
            }

            bebop_span_t labelSpan = bebop_diagnostic_label_span(diag, j);
            relatedInfo.push_back({
                .location = {.uri = doc.uri, .range = doc.spanToRange(labelSpan)},
                .message = labelMsg
            });

            std::string suggestion = extractSuggestion(labelMsg);
            if (!suggestion.empty()) {
                doc.diagnosticInfos.push_back({
                    .range = doc.spanToRange(labelSpan),
                    .hint = labelMsg,
                    .suggestion = std::move(suggestion)
                });
            }
        }

        if (!relatedInfo.empty()) {
            lspDiag.relatedInformation = std::move(relatedInfo);
        }

        lspDiag.source = "bebop";
        lspDiag.code = static_cast<int>(bebop_diagnostic_code(diag));

        doc.diagnostics.push_back(std::move(lspDiag));
    }
}

OpenDocument* Workspace::getDocument(const std::string& path)
{
    auto it = openDocuments_.find(path);
    return it != openDocuments_.end() ? &it->second : nullptr;
}

const OpenDocument* Workspace::getDocument(const std::string& path) const
{
    auto it = openDocuments_.find(path);
    return it != openDocuments_.end() ? &it->second : nullptr;
}

const std::string* Workspace::findImportForType(const std::string& typeName) const
{
    auto it = typeIndex_.find(typeName);
    if (it != typeIndex_.end()) {
        return &it->second;
    }
    return nullptr;
}

void Workspace::rebuildTypeIndex()
{
    typeIndexBuilt_ = false;
    typeIndex_.clear();
    scanIncludePathsForTypes();
}

void Workspace::scanIncludePathsForTypes()
{
    if (typeIndexBuilt_) return;
    typeIndexBuilt_ = true;

    for (const auto& includePath : includePaths_) {
        if (!fs::exists(includePath) || !fs::is_directory(includePath)) continue;

        for (auto& entry : fs::recursive_directory_iterator(includePath)) {
            if (!entry.is_regular_file()) continue;
            if (entry.path().extension() != ".bop") continue;

            std::string filePath = entry.path().string();
            std::string relativePath = fs::relative(entry.path(), includePath).string();

            // Quick parse to extract type names
            std::ifstream file(filePath, std::ios::binary | std::ios::ate);
            if (!file) continue;

            auto size = file.tellg();
            file.seekg(0, std::ios::beg);
            std::string content(static_cast<size_t>(size), '\0');
            file.read(content.data(), size);

            // Create a temporary context to parse
            bebop_host_t host = {};
            host.allocator.alloc = hostAlloc;
            host.file_reader.read = hostFileRead;
            host.file_reader.exists = hostFileExists;

            bebop_context_t* ctx = bebop_context_create(&host);
            if (!ctx) continue;

            bebop_source_t source = {
                .source = content.c_str(),
                .len = content.size(),
                .path = filePath.c_str()
            };

            bebop_parse_result_t* result = nullptr;
            bebop_status_t status = bebop_parse_source(ctx, &source, &result);

            if (status != BEBOP_FATAL && result) {
                uint32_t schemaCount = bebop_result_schema_count(result);
                for (uint32_t s = 0; s < schemaCount; ++s) {
                    const bebop_schema_t* schema = bebop_result_schema_at(result, s);
                    uint32_t defCount = bebop_schema_definition_count(schema);

                    for (uint32_t d = 0; d < defCount; ++d) {
                        const bebop_def_t* def = bebop_schema_definition_at(schema, d);
                        const char* name = bebop_def_name(def);
                        const char* fqn = bebop_def_fqn(def);

                        if (name) {
                            typeIndex_[name] = relativePath;
                        }
                        if (fqn) {
                            typeIndex_[fqn] = relativePath;
                        }
                    }
                }
            }

            bebop_context_destroy(ctx);
        }
    }
}

} // namespace beboplsp
