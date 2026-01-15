#include "server.hpp"
#include <algorithm>
#include <filesystem>
#include <map>
#include <set>
#include <sstream>

namespace beboplsp {

Server::Server(lsp::io::Stream& io, std::vector<std::string> includePaths)
    : connection_(io)
    , handler_(connection_)
    , workspace_(std::move(includePaths))
{
    registerHandlers();
}

void Server::run()
{
    running_ = true;
    while (running_) {
        handler_.processIncomingMessages();
    }
}

void Server::registerHandlers()
{
    handler_
        .add<lsp::requests::Initialize>(
            [this](auto&& params) { return onInitialize(std::move(params)); }
        )
        .add<lsp::notifications::Initialized>(
            [this](auto&& params) { onInitialized(std::move(params)); }
        )
        .add<lsp::requests::Shutdown>(
            [this]() { return onShutdown(); }
        )
        .add<lsp::notifications::Exit>(
            [this]() { onExit(); }
        )
        .add<lsp::notifications::TextDocument_DidOpen>(
            [this](auto&& params) { onDidOpen(std::move(params)); }
        )
        .add<lsp::notifications::TextDocument_DidChange>(
            [this](auto&& params) { onDidChange(std::move(params)); }
        )
        .add<lsp::notifications::TextDocument_DidClose>(
            [this](auto&& params) { onDidClose(std::move(params)); }
        )
        .add<lsp::notifications::TextDocument_DidSave>(
            [this](auto&& params) { onDidSave(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_Hover>(
            [this](auto&& params) { return onHover(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_Definition>(
            [this](auto&& params) { return onDefinition(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_References>(
            [this](auto&& params) { return onReferences(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_DocumentSymbol>(
            [this](auto&& params) { return onDocumentSymbol(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_Formatting>(
            [this](auto&& params) { return onFormatting(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_Completion>(
            [this](auto&& params) { return onCompletion(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_CodeAction>(
            [this](auto&& params) { return onCodeAction(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_RangeFormatting>(
            [this](auto&& params) { return onRangeFormatting(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_DocumentHighlight>(
            [this](auto&& params) { return onDocumentHighlight(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_FoldingRange>(
            [this](auto&& params) { return onFoldingRange(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_DocumentLink>(
            [this](auto&& params) { return onDocumentLink(std::move(params)); }
        )
        .add<lsp::requests::Workspace_Symbol>(
            [this](auto&& params) { return onWorkspaceSymbol(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_Rename>(
            [this](auto&& params) { return onRename(std::move(params)); }
        )
        .add<lsp::requests::TextDocument_PrepareRename>(
            [this](auto&& params) { return onPrepareRename(std::move(params)); }
        );
}

lsp::requests::Initialize::Result Server::onInitialize(
    lsp::requests::Initialize::Params&& params
)
{
    (void)params;

    return lsp::requests::Initialize::Result{
        .capabilities = {
            .positionEncoding = lsp::PositionEncodingKind::UTF16,
            .textDocumentSync = lsp::TextDocumentSyncOptions{
                .openClose = true,
                .change = lsp::TextDocumentSyncKind::Full,
                .save = lsp::SaveOptions{.includeText = false}
            },
            .completionProvider = lsp::CompletionOptions{},
            .hoverProvider = true,
            .definitionProvider = true,
            .referencesProvider = true,
            .documentHighlightProvider = true,
            .documentSymbolProvider = true,
            .documentFormattingProvider = true,
            .documentRangeFormattingProvider = true,
            .foldingRangeProvider = true,
            .renameProvider = lsp::RenameOptions{.prepareProvider = true},
            .documentLinkProvider = lsp::DocumentLinkOptions{},
            .codeActionProvider = true,
            .workspaceSymbolProvider = true,
        },
        .serverInfo = lsp::InitializeResultServerInfo{
            .name = "beboplsp",
            .version = BEBOP_VERSION_STRING
        },
    };
}

void Server::onInitialized(lsp::notifications::Initialized::Params&& params)
{
    (void)params;
    initialized_ = true;
}

lsp::requests::Shutdown::Result Server::onShutdown()
{
    return {};
}

void Server::onExit()
{
    running_ = false;
}

void Server::onDidOpen(lsp::notifications::TextDocument_DidOpen::Params&& params)
{
    workspace_.openDocument(params.textDocument.uri, std::move(params.textDocument.text));
    publishDiagnostics(params.textDocument.uri);
}

void Server::onDidChange(lsp::notifications::TextDocument_DidChange::Params&& params)
{
    for (auto& change : params.contentChanges) {
        if (auto* full = std::get_if<lsp::TextDocumentContentChangeEvent_Text>(&change)) {
            workspace_.updateDocument(params.textDocument.uri, full->text);
        }
    }
    publishDiagnostics(params.textDocument.uri);
}

void Server::onDidClose(lsp::notifications::TextDocument_DidClose::Params&& params)
{
    workspace_.closeDocument(params.textDocument.uri);
    handler_.sendNotification<lsp::notifications::TextDocument_PublishDiagnostics>(
        lsp::notifications::TextDocument_PublishDiagnostics::Params{
            .uri = params.textDocument.uri,
            .diagnostics = {}
        }
    );
}

void Server::onDidSave(lsp::notifications::TextDocument_DidSave::Params&& params)
{
    publishDiagnostics(params.textDocument.uri);
}

void Server::publishDiagnostics(const lsp::DocumentUri& uri)
{
    auto* doc = workspace_.getDocument(workspace_.uriToPath(uri));
    if (!doc) {
        return;
    }

    handler_.sendNotification<lsp::notifications::TextDocument_PublishDiagnostics>(
        lsp::notifications::TextDocument_PublishDiagnostics::Params{
            .uri = doc->uri,
            .diagnostics = doc->diagnostics
        }
    );
}

lsp::requests::TextDocument_Hover::Result Server::onHover(
    lsp::requests::TextDocument_Hover::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc || !doc->valid()) {
        return {};
    }

    uint32_t offset = doc->positionToOffset(params.position);
    bebop_location_t loc;
    if (!bebop_result_locate(doc->result, path.c_str(), offset, &loc)) {
        return {};
    }

    std::string content;
    lsp::Range range = doc->spanToRange(loc.span);

    switch (loc.kind) {
        case BEBOP_LOC_FIELD_TYPE: {
            const bebop_type_t* type = bebop_field_type(loc.field);
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                const bebop_def_t* typeDef = bebop_type_resolved(type);
                if (typeDef) {
                    content = formatDef(typeDef);
                    if (const char* docStr = bebop_def_documentation(typeDef)) {
                        content += "\n\n---\n\n";
                        content += docStr;
                    }
                    break;
                }
            }
            content = "```bebop\n" + formatType(type) + "\n```\n\nBuilt-in type";
            break;
        }

        case BEBOP_LOC_FIELD_NAME: {
            content = "```bebop\n";
            content += bebop_field_name(loc.field);
            content += ": ";
            content += formatType(bebop_field_type(loc.field));
            content += "\n```";
            break;
        }

        case BEBOP_LOC_BRANCH:
            content = formatBranch(loc.branch);
            break;

        case BEBOP_LOC_MEMBER:
            content = formatMember(loc.member);
            break;

        case BEBOP_LOC_TYPE_REF: {
            const bebop_type_t* type = loc.type;
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                const bebop_def_t* typeDef = bebop_type_resolved(type);
                if (typeDef) {
                    content = formatDef(typeDef);
                    if (const char* docStr = bebop_def_documentation(typeDef)) {
                        content += "\n\n---\n\n";
                        content += docStr;
                    }
                    break;
                }
            }
            content = "```bebop\n" + formatType(type) + "\n```";
            break;
        }

        case BEBOP_LOC_DECORATOR: {
            const bebop_def_t* decoratorDef = bebop_decorator_resolved(loc.decorator);
            if (decoratorDef) {
                content = formatDecoratorDef(decoratorDef);
                if (const char* docStr = bebop_def_documentation(decoratorDef)) {
                    content += "\n\n---\n\n";
                    content += docStr;
                }
            } else {
                content = "```bebop\n@";
                content += bebop_decorator_name(loc.decorator);
                content += "\n```";
            }
            break;
        }

        case BEBOP_LOC_METHOD: {
            content = formatMethod(loc.method);
            if (const char* docStr = bebop_method_documentation(loc.method)) {
                content += "\n\n---\n\n";
                content += docStr;
            }
            break;
        }

        case BEBOP_LOC_MIXIN: {
            const bebop_type_t* type = loc.type;
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                const bebop_def_t* mixinDef = bebop_type_resolved(type);
                if (mixinDef) {
                    content = formatDef(mixinDef);
                    if (const char* docStr = bebop_def_documentation(mixinDef)) {
                        content += "\n\n---\n\n";
                        content += docStr;
                    }
                    break;
                }
            }
            content = "```bebop\n" + formatType(type) + "\n```";
            break;
        }

        case BEBOP_LOC_DEF: {
            bebop_def_kind_t kind = bebop_def_kind(loc.def);
            content = (kind == BEBOP_DEF_DECORATOR)
                ? formatDecoratorDef(loc.def)
                : formatDef(loc.def);
            if (const char* docStr = bebop_def_documentation(loc.def)) {
                content += "\n\n---\n\n";
                content += docStr;
            }
            break;
        }

        default:
            return {};
    }

    return lsp::Hover{
        .contents = lsp::MarkupContent{
            .kind = lsp::MarkupKind::Markdown,
            .value = std::move(content)
        },
        .range = range
    };
}

lsp::requests::TextDocument_Definition::Result Server::onDefinition(
    lsp::requests::TextDocument_Definition::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc || !doc->valid()) {
        return {};
    }

    uint32_t offset = doc->positionToOffset(params.position);
    bebop_location_t loc;
    if (!bebop_result_locate(doc->result, path.c_str(), offset, &loc)) {
        return {};
    }

    const bebop_def_t* targetDef = nullptr;

    switch (loc.kind) {
        case BEBOP_LOC_FIELD_TYPE: {
            const bebop_type_t* type = bebop_field_type(loc.field);
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                targetDef = bebop_type_resolved(type);
            }
            break;
        }

        case BEBOP_LOC_TYPE_REF: {
            const bebop_type_t* type = loc.type;
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                targetDef = bebop_type_resolved(type);
            }
            break;
        }

        case BEBOP_LOC_DECORATOR:
            targetDef = bebop_decorator_resolved(loc.decorator);
            break;

        case BEBOP_LOC_MIXIN: {
            const bebop_type_t* type = loc.type;
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                targetDef = bebop_type_resolved(type);
            }
            break;
        }

        case BEBOP_LOC_BRANCH:
            if (loc.def) {
                targetDef = loc.def;
            } else if (loc.branch) {
                const bebop_type_t* typeRef = bebop_branch_type_ref(loc.branch);
                if (typeRef && bebop_type_kind(typeRef) == BEBOP_TYPE_DEFINED) {
                    targetDef = bebop_type_resolved(typeRef);
                }
            }
            break;

        case BEBOP_LOC_DEF:
            targetDef = loc.def;
            break;

        default:
            return {};
    }

    if (!targetDef) {
        return {};
    }

    const bebop_schema_t* schema = bebop_def_schema(targetDef);
    const char* schemaPath = bebop_schema_path(schema);
    std::string defPath = schemaPath ? schemaPath : path;

    auto* targetDoc = workspace_.getDocument(defPath);
    lsp::Location result;
    result.uri = lsp::DocumentUri::fromPath(defPath);
    result.range = targetDoc
        ? targetDoc->spanToRange(bebop_def_span(targetDef))
        : doc->spanToRange(bebop_def_span(targetDef));

    return result;
}

lsp::requests::TextDocument_References::Result Server::onReferences(
    lsp::requests::TextDocument_References::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc || !doc->valid()) {
        return {};
    }

    uint32_t offset = doc->positionToOffset(params.position);
    bebop_location_t loc;
    if (!bebop_result_locate(doc->result, path.c_str(), offset, &loc)) {
        return {};
    }

    const bebop_def_t* def = nullptr;

    switch (loc.kind) {
        case BEBOP_LOC_FIELD_TYPE: {
            const bebop_type_t* type = bebop_field_type(loc.field);
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                def = bebop_type_resolved(type);
            }
            break;
        }

        case BEBOP_LOC_TYPE_REF: {
            const bebop_type_t* type = loc.type;
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                def = bebop_type_resolved(type);
            }
            break;
        }

        case BEBOP_LOC_DECORATOR:
            def = bebop_decorator_resolved(loc.decorator);
            break;

        case BEBOP_LOC_MIXIN: {
            const bebop_type_t* type = loc.type;
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                def = bebop_type_resolved(type);
            }
            break;
        }

        case BEBOP_LOC_DEF:
            def = loc.def;
            break;

        default:
            return {};
    }

    if (!def) {
        return {};
    }

    const char* targetFqn = bebop_def_fqn(def);
    if (!targetFqn) {
        return {};
    }

    std::vector<lsp::Location> locations;

    if (params.context.includeDeclaration) {
        const bebop_schema_t* defSchema = bebop_def_schema(def);
        const char* defSchemaPath = bebop_schema_path(defSchema);
        std::string defPathStr = defSchemaPath ? defSchemaPath : path;

        auto* targetDoc = workspace_.getDocument(defPathStr);
        lsp::Location defLoc;
        defLoc.uri = lsp::DocumentUri::fromPath(defPathStr);
        defLoc.range = targetDoc
            ? targetDoc->spanToRange(bebop_def_span(def))
            : doc->spanToRange(bebop_def_span(def));
        locations.push_back(std::move(defLoc));
    }

    workspace_.forEachDocument([&](const std::string&, OpenDocument& otherDoc) {
        if (!otherDoc.valid()) {
            return;
        }

        uint32_t defCount = bebop_result_definition_count(otherDoc.result);
        for (uint32_t d = 0; d < defCount; ++d) {
            const bebop_def_t* otherDef = bebop_result_definition_at(otherDoc.result, d);
            const char* otherFqn = bebop_def_fqn(otherDef);
            if (!otherFqn || strcmp(otherFqn, targetFqn) != 0) {
                continue;
            }

            uint32_t refCount = bebop_def_references_count(otherDef);
            for (uint32_t i = 0; i < refCount; ++i) {
                bebop_span_t span = bebop_def_reference_at(otherDef, i);
                locations.push_back(lsp::Location{
                    .uri = otherDoc.uri,
                    .range = otherDoc.spanToRange(span)
                });
            }
        }
    });

    return locations;
}

lsp::requests::TextDocument_DocumentSymbol::Result Server::onDocumentSymbol(
    lsp::requests::TextDocument_DocumentSymbol::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc || !doc->valid()) {
        return {};
    }

    std::vector<lsp::DocumentSymbol> symbols;

    uint32_t schemaCount = bebop_result_schema_count(doc->result);
    for (uint32_t s = 0; s < schemaCount; ++s) {
        const bebop_schema_t* schema = bebop_result_schema_at(doc->result, s);
        const char* schemaPath = bebop_schema_path(schema);

        if (!schemaPath || path != schemaPath) {
            continue;
        }

        uint32_t defCount = bebop_schema_definition_count(schema);
        for (uint32_t d = 0; d < defCount; ++d) {
            const bebop_def_t* def = bebop_schema_definition_at(schema, d);

            lsp::DocumentSymbol sym;
            sym.name = bebop_def_name(def);
            sym.kind = defKindToSymbolKind(bebop_def_kind(def));
            sym.range = doc->spanToRange(bebop_def_span(def));
            sym.selectionRange = sym.range;

            uint32_t fieldCount = bebop_def_field_count(def);
            if (fieldCount > 0) {
                sym.children = std::vector<lsp::DocumentSymbol>{};
                for (uint32_t f = 0; f < fieldCount; ++f) {
                    const bebop_field_t* field = bebop_def_field_at(def, f);
                    lsp::DocumentSymbol fieldSym;
                    fieldSym.name = bebop_field_name(field);
                    fieldSym.kind = lsp::SymbolKind::Field;
                    fieldSym.range = doc->spanToRange(bebop_field_span(field));
                    fieldSym.selectionRange = doc->spanToRange(bebop_field_name_span(field));
                    sym.children->push_back(std::move(fieldSym));
                }
            }

            uint32_t memberCount = bebop_def_member_count(def);
            if (memberCount > 0) {
                sym.children = std::vector<lsp::DocumentSymbol>{};
                for (uint32_t m = 0; m < memberCount; ++m) {
                    const bebop_enum_member_t* member = bebop_def_member_at(def, m);
                    lsp::DocumentSymbol memberSym;
                    memberSym.name = bebop_member_name(member);
                    memberSym.kind = lsp::SymbolKind::EnumMember;
                    memberSym.range = doc->spanToRange(bebop_member_span(member));
                    memberSym.selectionRange = memberSym.range;
                    sym.children->push_back(std::move(memberSym));
                }
            }

            symbols.push_back(std::move(sym));
        }
    }

    return symbols;
}

lsp::requests::TextDocument_Formatting::Result Server::onFormatting(
    lsp::requests::TextDocument_Formatting::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc || !doc->valid()) {
        return {};
    }

    std::string formatted;

    uint32_t schemaCount = bebop_result_schema_count(doc->result);
    for (uint32_t s = 0; s < schemaCount; ++s) {
        const bebop_schema_t* schema = bebop_result_schema_at(doc->result, s);
        const char* schemaPath = bebop_schema_path(schema);

        if (!schemaPath || path != schemaPath) {
            continue;
        }

        size_t len = 0;
        const char* emitted = bebop_emit_schema(schema, &len);
        if (emitted && len > 0) {
            if (!formatted.empty()) {
                formatted += "\n";
            }
            formatted.append(emitted, len);
        }
    }

    if (formatted.empty()) {
        return {};
    }

    lsp::Range fullRange{
        .start = {0, 0},
        .end = doc->offsetToPosition(static_cast<uint32_t>(doc->content.size()))
    };

    return std::vector<lsp::TextEdit>{{
        .range = fullRange,
        .newText = std::move(formatted)
    }};
}

lsp::requests::TextDocument_Completion::Result Server::onCompletion(
    lsp::requests::TextDocument_Completion::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    std::vector<lsp::CompletionItem> items;

    static const char* keywords[] = {
        "enum", "struct", "message", "union", "service", "const",
        "mut", "export", "local", "map", "array", "stream",
        "import", "edition", "package", "true", "false"
    };
    for (const char* kw : keywords) {
        items.push_back({.label = kw, .kind = lsp::CompletionItemKind::Keyword});
    }

    static const char* primitives[] = {
        "bool", "byte", "string",
        "int8", "int16", "int32", "int64", "int128",
        "uint16", "uint32", "uint64", "uint128",
        "float16", "float32", "float64", "bfloat16",
        "uuid", "timestamp", "duration"
    };
    for (const char* prim : primitives) {
        items.push_back({.label = prim, .kind = lsp::CompletionItemKind::TypeParameter});
    }

    static const struct { const char* alias; const char* canonical; } aliases[] = {
        {"uint8", "byte"},
        {"guid", "uuid"},
        {"half", "float16"},
        {"bf16", "bfloat16"}
    };
    for (const auto& [alias, canonical] : aliases) {
        lsp::CompletionItem item;
        item.label = alias;
        item.kind = lsp::CompletionItemKind::TypeParameter;
        item.detail = std::string("alias for ") + canonical;
        items.push_back(std::move(item));
    }

    auto* doc = workspace_.getDocument(path);
    if (doc && doc->valid()) {
        uint32_t schemaCount = bebop_result_schema_count(doc->result);
        for (uint32_t s = 0; s < schemaCount; ++s) {
            const bebop_schema_t* schema = bebop_result_schema_at(doc->result, s);
            uint32_t defCount = bebop_schema_definition_count(schema);
            for (uint32_t d = 0; d < defCount; ++d) {
                const bebop_def_t* def = bebop_schema_definition_at(schema, d);
                items.push_back({
                    .label = bebop_def_name(def),
                    .kind = defKindToCompletionKind(bebop_def_kind(def))
                });
            }
        }
    }

    return items;
}

lsp::requests::TextDocument_CodeAction::Result Server::onCodeAction(
    lsp::requests::TextDocument_CodeAction::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc) {
        return {};
    }

    std::vector<std::variant<lsp::Command, lsp::CodeAction>> results;
    std::set<std::string> seenActions; // Deduplicate by title

    // Quick fixes for "did you mean" suggestions
    for (const auto& info : doc->diagnosticInfos) {
        if (info.suggestion.empty()) {
            continue;
        }

        bool overlaps =
            !(info.range.end.line < params.range.start.line ||
              info.range.start.line > params.range.end.line ||
              (info.range.end.line == params.range.start.line &&
               info.range.end.character < params.range.start.character) ||
              (info.range.start.line == params.range.end.line &&
               info.range.start.character > params.range.end.character));

        if (overlaps) {
            std::string title = "Replace with '" + info.suggestion + "'";
            if (!seenActions.insert(title).second) continue; // Skip duplicate

            lsp::CodeAction action;
            action.title = std::move(title);
            action.kind = lsp::CodeActionKind::QuickFix;
            action.isPreferred = true;

            lsp::WorkspaceEdit edit;
            lsp::Map<lsp::DocumentUri, lsp::Array<lsp::TextEdit>> changes;
            changes[doc->uri].push_back({
                .range = info.range,
                .newText = info.suggestion
            });
            edit.changes = std::move(changes);
            action.edit = std::move(edit);

            results.push_back(std::move(action));
        }
    }

    // Import suggestions for unknown types (error code 105)
    for (const auto& diag : doc->diagnostics) {
        if (!diag.code.has_value()) continue;
        auto* codeInt = std::get_if<int>(&*diag.code);
        if (!codeInt || *codeInt != 105) continue;

        bool overlaps =
            !(diag.range.end.line < params.range.start.line ||
              diag.range.start.line > params.range.end.line ||
              (diag.range.end.line == params.range.start.line &&
               diag.range.end.character < params.range.start.character) ||
              (diag.range.start.line == params.range.end.line &&
               diag.range.start.character > params.range.end.character));

        if (!overlaps) continue;

        // Extract the type name from the diagnostic range
        uint32_t startOff = doc->positionToOffset(diag.range.start);
        uint32_t endOff = doc->positionToOffset(diag.range.end);
        if (endOff <= startOff || endOff > doc->content.size()) continue;

        std::string typeName = doc->content.substr(startOff, endOff - startOff);

        // Look up in the type index (scans include paths lazily)
        const_cast<Workspace&>(workspace_).rebuildTypeIndex();
        if (const std::string* importPath = workspace_.findImportForType(typeName)) {
            std::string title = "Add import \"" + *importPath + "\"";
            if (!seenActions.insert(title).second) continue; // Skip duplicate

            // Find position to insert import (after existing imports or at top)
            lsp::Position insertPos = {0, 0};
            size_t searchPos = 0;
            while ((searchPos = doc->content.find("import", searchPos)) != std::string::npos) {
                size_t lineEnd = doc->content.find('\n', searchPos);
                if (lineEnd != std::string::npos) {
                    insertPos = doc->offsetToPosition(static_cast<uint32_t>(lineEnd + 1));
                }
                searchPos += 6;
            }

            lsp::CodeAction action;
            action.title = std::move(title);
            action.kind = lsp::CodeActionKind::QuickFix;

            lsp::WorkspaceEdit edit;
            lsp::Map<lsp::DocumentUri, lsp::Array<lsp::TextEdit>> changes;
            changes[doc->uri].push_back({
                .range = {insertPos, insertPos},
                .newText = "import \"" + *importPath + "\"\n"
            });
            edit.changes = std::move(changes);
            action.edit = std::move(edit);

            results.push_back(std::move(action));
        }
    }

    return results;
}

lsp::requests::TextDocument_RangeFormatting::Result Server::onRangeFormatting(
    lsp::requests::TextDocument_RangeFormatting::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc || !doc->valid()) {
        return {};
    }

    std::string formatted;
    lsp::Range formattedRange = params.range;

    uint32_t schemaCount = bebop_result_schema_count(doc->result);
    for (uint32_t s = 0; s < schemaCount; ++s) {
        const bebop_schema_t* schema = bebop_result_schema_at(doc->result, s);
        const char* schemaPath = bebop_schema_path(schema);

        if (!schemaPath || path != schemaPath) {
            continue;
        }

        uint32_t defCount = bebop_schema_definition_count(schema);
        for (uint32_t d = 0; d < defCount; ++d) {
            const bebop_def_t* def = bebop_schema_definition_at(schema, d);

            // Nested definitions are emitted by their parent
            if (bebop_def_parent(def) != nullptr) {
                continue;
            }

            lsp::Range defRange = doc->spanToRange(bebop_def_span(def));

            bool overlaps =
                !(defRange.end.line < params.range.start.line ||
                  defRange.start.line > params.range.end.line);

            if (overlaps) {
                size_t len = 0;
                const char* emitted = bebop_emit_def(def, &len);
                if (emitted && len > 0) {
                    if (!formatted.empty()) {
                        formatted += "\n\n";
                    } else {
                        formattedRange.start = defRange.start;
                    }
                    formatted.append(emitted, len);
                    formattedRange.end = defRange.end;
                }
            }
        }
    }

    if (formatted.empty()) {
        return {};
    }

    return std::vector<lsp::TextEdit>{{
        .range = formattedRange,
        .newText = std::move(formatted)
    }};
}

std::string Server::formatType(const bebop_type_t* type, bebop_context_t* ctx)
{
    if (!type) {
        return "?";
    }

    bebop_type_kind_t kind = bebop_type_kind(type);

    switch (kind) {
        case BEBOP_TYPE_BOOL:      return "bool";
        case BEBOP_TYPE_BYTE:      return "byte";
        case BEBOP_TYPE_INT8:      return "int8";
        case BEBOP_TYPE_INT16:     return "int16";
        case BEBOP_TYPE_UINT16:    return "uint16";
        case BEBOP_TYPE_INT32:     return "int32";
        case BEBOP_TYPE_UINT32:    return "uint32";
        case BEBOP_TYPE_INT64:     return "int64";
        case BEBOP_TYPE_UINT64:    return "uint64";
        case BEBOP_TYPE_INT128:    return "int128";
        case BEBOP_TYPE_UINT128:   return "uint128";
        case BEBOP_TYPE_FLOAT16:   return "float16";
        case BEBOP_TYPE_FLOAT32:   return "float32";
        case BEBOP_TYPE_FLOAT64:   return "float64";
        case BEBOP_TYPE_BFLOAT16:  return "bfloat16";
        case BEBOP_TYPE_STRING:    return "string";
        case BEBOP_TYPE_UUID:      return "uuid";
        case BEBOP_TYPE_TIMESTAMP: return "timestamp";
        case BEBOP_TYPE_DURATION:  return "duration";

        case BEBOP_TYPE_ARRAY:
            return formatType(bebop_type_element(type), ctx) + "[]";

        case BEBOP_TYPE_FIXED_ARRAY: {
            std::ostringstream oss;
            oss << formatType(bebop_type_element(type), ctx)
                << "[" << bebop_type_fixed_array_size(type) << "]";
            return oss.str();
        }

        case BEBOP_TYPE_MAP:
            return "map[" + formatType(bebop_type_key(type), ctx) + ", "
                   + formatType(bebop_type_value(type), ctx) + "]";

        case BEBOP_TYPE_DEFINED: {
            const char* name = ctx ? bebop_type_name_in_ctx(type, ctx) : bebop_type_name(type);
            return name ? name : "?";
        }

        default:
            return bebop_type_kind_name(kind);
    }
}

std::string Server::formatDef(const bebop_def_t* def, bebop_context_t* ctx)
{
    std::ostringstream oss;
    bebop_def_kind_t kind = bebop_def_kind(def);

    // Get context from def's schema if not provided
    if (!ctx && def) {
        const bebop_schema_t* schema = bebop_def_schema(def);
        if (schema) {
            ctx = bebop_schema_context(schema);
        }
    }

    oss << "```bebop\n"
        << bebop_def_kind_name(kind) << " " << bebop_def_name(def);

    if (kind == BEBOP_DEF_SERVICE) {
        uint32_t mixinCount = bebop_def_mixin_count(def);
        if (mixinCount > 0) {
            oss << " with ";
            for (uint32_t i = 0; i < mixinCount; ++i) {
                if (i > 0) oss << ", ";
                oss << formatType(bebop_def_mixin_at(def, i), ctx);
            }
        }
    }

    oss << "\n```";

    if (kind == BEBOP_DEF_STRUCT || kind == BEBOP_DEF_MESSAGE ||
        kind == BEBOP_DEF_UNION || kind == BEBOP_DEF_ENUM) {
        oss << "\n\n";
        uint32_t fieldCount = bebop_def_field_count(def);
        if (bebop_def_is_fixed_size(def)) {
            uint32_t size = bebop_def_fixed_size(def);
            if (size == 0 && fieldCount == 0) {
                oss << "**Size:** 0 bytes (empty)";
            } else {
                oss << "**Size:** " << size << " byte" << (size != 1 ? "s" : "") << " (fixed)";
            }
        } else {
            uint32_t minSize = bebop_def_min_wire_size(def);
            oss << "**Size:** " << minSize << "+ byte" << (minSize != 1 ? "s" : "") << " (variable)";
        }

        if (fieldCount > 0) {
            oss << " · " << fieldCount << " field" << (fieldCount != 1 ? "s" : "");
        }
    } else if (kind == BEBOP_DEF_SERVICE) {
        uint32_t methodCount = bebop_def_method_count(def);
        uint32_t mixinCount = bebop_def_mixin_count(def);

        // Collect all methods: own + inherited from mixins
        struct MethodInfo {
            const bebop_method_t* method;
            const char* source; // NULL for own methods, mixin name for inherited
        };
        std::vector<MethodInfo> allMethods;

        // Own methods
        for (uint32_t i = 0; i < methodCount; ++i) {
            allMethods.push_back({bebop_def_method_at(def, i), nullptr});
        }

        // Inherited methods from mixins
        for (uint32_t m = 0; m < mixinCount; ++m) {
            const bebop_type_t* mixinType = bebop_def_mixin_at(def, m);
            const bebop_def_t* mixinDef = bebop_type_resolved(mixinType);
            if (!mixinDef) continue;

            const char* mixinName = bebop_def_name(mixinDef);
            uint32_t mixinMethodCount = bebop_def_method_count(mixinDef);
            for (uint32_t i = 0; i < mixinMethodCount; ++i) {
                allMethods.push_back({bebop_def_method_at(mixinDef, i), mixinName});
            }
        }

        if (!allMethods.empty()) {
            oss << "\n\n| Method | Request | Response | Source |\n|--------|---------|----------|--------|\n";
            for (const auto& info : allMethods) {
                const bebop_method_t* method = info.method;
                bebop_method_type_t mtype = bebop_method_type(method);

                oss << "| `" << bebop_method_name(method) << "` | ";

                if (const bebop_type_t* req = bebop_method_request_type(method)) {
                    if (mtype == BEBOP_METHOD_CLIENT_STREAM || mtype == BEBOP_METHOD_DUPLEX_STREAM) {
                        oss << "stream ";
                    }
                    oss << "`" << formatType(req, ctx) << "`";
                } else {
                    oss << "—";
                }

                oss << " | ";

                if (const bebop_type_t* resp = bebop_method_response_type(method)) {
                    if (mtype == BEBOP_METHOD_SERVER_STREAM || mtype == BEBOP_METHOD_DUPLEX_STREAM) {
                        oss << "stream ";
                    }
                    oss << "`" << formatType(resp, ctx) << "`";
                } else {
                    oss << "—";
                }

                oss << " | ";
                if (info.source) {
                    oss << "*" << info.source << "*";
                } else {
                    oss << "—";
                }
                oss << " |\n";
            }
        } else {
            oss << "\n\n*No methods*";
        }
    }

    return oss.str();
}

std::string Server::formatBranch(const bebop_union_branch_t* branch, bebop_context_t* ctx)
{
    std::ostringstream oss;
    oss << "```bebop\n";

    if (const bebop_def_t* parent = bebop_branch_parent(branch)) {
        oss << "(union " << bebop_def_name(parent) << ")\n";
        if (!ctx) {
            const bebop_schema_t* schema = bebop_def_schema(parent);
            if (schema) ctx = bebop_schema_context(schema);
        }
    }

    const bebop_def_t* branchDef = bebop_branch_def(branch);
    const bebop_type_t* typeRef = bebop_branch_type_ref(branch);

    if (branchDef) {
        oss << bebop_def_name(branchDef) << "(" << static_cast<int>(bebop_branch_discriminator(branch)) << "): "
            << bebop_def_kind_name(bebop_def_kind(branchDef));
    } else if (typeRef) {
        if (const char* name = bebop_branch_name(branch)) {
            oss << name;
        }
        oss << "(" << static_cast<int>(bebop_branch_discriminator(branch)) << "): " << formatType(typeRef, ctx);
    }

    oss << "\n```";

    if (branchDef) {
        oss << "\n\n";
        uint32_t fieldCount = bebop_def_field_count(branchDef);
        if (bebop_def_is_fixed_size(branchDef)) {
            uint32_t size = bebop_def_fixed_size(branchDef);
            if (size == 0 && fieldCount == 0) {
                oss << "**Size:** 0 bytes (empty)";
            } else {
                oss << "**Size:** " << size << " byte" << (size != 1 ? "s" : "") << " (fixed)";
            }
        } else {
            uint32_t minSize = bebop_def_min_wire_size(branchDef);
            oss << "**Size:** " << minSize << "+ byte" << (minSize != 1 ? "s" : "") << " (variable)";
        }
        if (fieldCount > 0) {
            oss << " · " << fieldCount << " field" << (fieldCount != 1 ? "s" : "");
        }
    }

    return oss.str();
}

std::string Server::formatMember(const bebop_enum_member_t* member)
{
    std::ostringstream oss;
    oss << "```bebop\n";

    if (const bebop_def_t* parent = bebop_member_parent(member)) {
        oss << "(enum " << bebop_def_name(parent) << ")\n";
    }

    oss << bebop_member_name(member) << " = ";

    if (const char* expr = bebop_member_value_expr(member)) {
        oss << expr;
    } else {
        oss << bebop_member_value(member);
    }

    oss << "\n```";
    return oss.str();
}

std::string Server::formatMethod(const bebop_method_t* method, bebop_context_t* ctx)
{
    std::ostringstream oss;

    const bebop_def_t* parent = bebop_method_parent(method);
    const char* methodName = bebop_method_name(method);
    bebop_method_type_t methodType = bebop_method_type(method);

    // Get context from parent service if not provided
    if (!ctx && parent) {
        const bebop_schema_t* schema = bebop_def_schema(parent);
        if (schema) ctx = bebop_schema_context(schema);
    }

    oss << "**" << methodName << "**\n\n";

    if (const bebop_type_t* reqType = bebop_method_request_type(method)) {
        oss << "**Request:** ";
        if (methodType == BEBOP_METHOD_CLIENT_STREAM || methodType == BEBOP_METHOD_DUPLEX_STREAM) {
            oss << "`stream` ";
        }
        oss << "`" << formatType(reqType, ctx) << "`\n\n";
    }

    if (const bebop_type_t* respType = bebop_method_response_type(method)) {
        oss << "**Response:** ";
        if (methodType == BEBOP_METHOD_SERVER_STREAM || methodType == BEBOP_METHOD_DUPLEX_STREAM) {
            oss << "`stream` ";
        }
        oss << "`" << formatType(respType, ctx) << "`\n\n";
    }

    if (parent) {
        oss << "service `" << bebop_def_name(parent) << "` · ";
    }
    oss << "ID: `0x" << std::hex << bebop_method_id(method) << std::dec << "`";

    return oss.str();
}

std::string Server::formatDecoratorDef(const bebop_def_t* def)
{
    std::ostringstream oss;

    oss << "```bebop\n@" << bebop_def_name(def) << "\n```\ndecorator";

    if (bebop_def_decorator_allow_multiple(def)) {
        oss << " (allow multiple)";
    }

    uint32_t paramCount = bebop_def_decorator_param_count(def);
    if (paramCount > 0) {
        oss << "\n\n**Parameters:**\n";
        for (uint32_t i = 0; i < paramCount; ++i) {
            const char* paramName = bebop_def_decorator_param_name(def, i);
            bool required = bebop_def_decorator_param_required(def, i);
            bebop_type_kind_t paramType = bebop_def_decorator_param_type(def, i);
            const char* desc = bebop_def_decorator_param_description(def, i);

            oss << "- `" << paramName << (required ? "!" : "?") << "`: "
                << bebop_type_kind_name(paramType);
            if (desc && desc[0]) {
                oss << " — " << desc;
            }
            oss << "\n";
        }
    }

    bebop_decorator_target_t targets = bebop_def_decorator_targets(def);
    if (targets != BEBOP_TARGET_NONE) {
        oss << "\n**Targets:** ";
        bool first = true;
        auto appendTarget = [&](bebop_decorator_target_t flag, const char* name) {
            if (targets & flag) {
                oss << (first ? "" : ", ") << name;
                first = false;
            }
        };
        appendTarget(BEBOP_TARGET_ENUM, "enum");
        appendTarget(BEBOP_TARGET_STRUCT, "struct");
        appendTarget(BEBOP_TARGET_MESSAGE, "message");
        appendTarget(BEBOP_TARGET_UNION, "union");
        appendTarget(BEBOP_TARGET_FIELD, "field");
        appendTarget(BEBOP_TARGET_SERVICE, "service");
        appendTarget(BEBOP_TARGET_METHOD, "method");
        appendTarget(BEBOP_TARGET_BRANCH, "branch");
    }

    return oss.str();
}

lsp::SymbolKind Server::defKindToSymbolKind(bebop_def_kind_t kind)
{
    switch (kind) {
        case BEBOP_DEF_ENUM:      return lsp::SymbolKind::Enum;
        case BEBOP_DEF_STRUCT:    return lsp::SymbolKind::Struct;
        case BEBOP_DEF_MESSAGE:   return lsp::SymbolKind::Struct;
        case BEBOP_DEF_UNION:     return lsp::SymbolKind::Struct;
        case BEBOP_DEF_SERVICE:   return lsp::SymbolKind::Interface;
        case BEBOP_DEF_CONST:     return lsp::SymbolKind::Constant;
        case BEBOP_DEF_DECORATOR: return lsp::SymbolKind::Function;
        default:                  return lsp::SymbolKind::Class;
    }
}

lsp::CompletionItemKind Server::defKindToCompletionKind(bebop_def_kind_t kind)
{
    switch (kind) {
        case BEBOP_DEF_ENUM:      return lsp::CompletionItemKind::Enum;
        case BEBOP_DEF_STRUCT:    return lsp::CompletionItemKind::Struct;
        case BEBOP_DEF_MESSAGE:   return lsp::CompletionItemKind::Struct;
        case BEBOP_DEF_UNION:     return lsp::CompletionItemKind::Struct;
        case BEBOP_DEF_SERVICE:   return lsp::CompletionItemKind::Interface;
        case BEBOP_DEF_CONST:     return lsp::CompletionItemKind::Constant;
        case BEBOP_DEF_DECORATOR: return lsp::CompletionItemKind::Function;
        default:                  return lsp::CompletionItemKind::Class;
    }
}

lsp::requests::TextDocument_DocumentHighlight::Result Server::onDocumentHighlight(
    lsp::requests::TextDocument_DocumentHighlight::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc || !doc->valid()) {
        return {};
    }

    uint32_t offset = doc->positionToOffset(params.position);
    bebop_location_t loc;
    if (!bebop_result_locate(doc->result, path.c_str(), offset, &loc)) {
        return {};
    }

    const bebop_def_t* def = nullptr;

    switch (loc.kind) {
        case BEBOP_LOC_FIELD_TYPE: {
            const bebop_type_t* type = bebop_field_type(loc.field);
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                def = bebop_type_resolved(type);
            }
            break;
        }
        case BEBOP_LOC_TYPE_REF: {
            const bebop_type_t* type = loc.type;
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                def = bebop_type_resolved(type);
            }
            break;
        }
        case BEBOP_LOC_DECORATOR:
            def = bebop_decorator_resolved(loc.decorator);
            break;
        case BEBOP_LOC_MIXIN: {
            const bebop_type_t* type = loc.type;
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                def = bebop_type_resolved(type);
            }
            break;
        }
        case BEBOP_LOC_DEF:
            def = loc.def;
            break;
        default:
            return {};
    }

    if (!def) {
        return {};
    }

    std::vector<lsp::DocumentHighlight> highlights;

    highlights.push_back({
        .range = doc->spanToRange(bebop_def_span(def)),
        .kind = lsp::DocumentHighlightKind::Text
    });

    uint32_t refCount = bebop_def_references_count(def);
    for (uint32_t i = 0; i < refCount; ++i) {
        highlights.push_back({
            .range = doc->spanToRange(bebop_def_reference_at(def, i)),
            .kind = lsp::DocumentHighlightKind::Read
        });
    }

    return highlights;
}

lsp::requests::TextDocument_FoldingRange::Result Server::onFoldingRange(
    lsp::requests::TextDocument_FoldingRange::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc || !doc->valid()) {
        return {};
    }

    std::vector<lsp::FoldingRange> ranges;

    uint32_t schemaCount = bebop_result_schema_count(doc->result);
    for (uint32_t s = 0; s < schemaCount; ++s) {
        const bebop_schema_t* schema = bebop_result_schema_at(doc->result, s);
        const char* schemaPath = bebop_schema_path(schema);

        if (!schemaPath || path != schemaPath) {
            continue;
        }

        uint32_t defCount = bebop_schema_definition_count(schema);
        for (uint32_t d = 0; d < defCount; ++d) {
            const bebop_def_t* def = bebop_schema_definition_at(schema, d);
            lsp::Range range = doc->spanToRange(bebop_def_span(def));

            if (range.start.line < range.end.line) {
                ranges.push_back({
                    .startLine = range.start.line,
                    .endLine = range.end.line,
                    .kind = lsp::FoldingRangeKind::Region
                });
            }
        }
    }

    return ranges;
}

lsp::requests::TextDocument_DocumentLink::Result Server::onDocumentLink(
    lsp::requests::TextDocument_DocumentLink::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc) {
        return {};
    }

    std::vector<lsp::DocumentLink> links;
    const std::string& content = doc->content;

    size_t pos = 0;
    while ((pos = content.find("import", pos)) != std::string::npos) {
        size_t lineStart = content.rfind('\n', pos);
        lineStart = (lineStart == std::string::npos) ? 0 : lineStart + 1;

        // Skip if not at start of line (after optional whitespace)
        bool atLineStart = true;
        for (size_t i = lineStart; i < pos; ++i) {
            if (content[i] != ' ' && content[i] != '\t') {
                atLineStart = false;
                break;
            }
        }

        if (!atLineStart) {
            pos += 6;
            continue;
        }

        size_t quoteStart = content.find('"', pos + 6);
        if (quoteStart == std::string::npos) {
            pos += 6;
            continue;
        }

        size_t quoteEnd = content.find('"', quoteStart + 1);
        if (quoteEnd == std::string::npos) {
            pos += 6;
            continue;
        }

        std::string importPath = content.substr(quoteStart + 1, quoteEnd - quoteStart - 1);

        lsp::DocumentLink link;
        link.range = {
            doc->offsetToPosition(static_cast<uint32_t>(quoteStart)),
            doc->offsetToPosition(static_cast<uint32_t>(quoteEnd + 1))
        };

        for (const auto& incPath : workspace_.includePaths()) {
            std::filesystem::path resolved = std::filesystem::path(incPath) / importPath;
            if (std::filesystem::exists(resolved)) {
                link.target = lsp::DocumentUri::fromPath(std::filesystem::canonical(resolved).string());
                links.push_back(std::move(link));
                break;
            }
        }

        pos = quoteEnd + 1;
    }

    return links;
}

lsp::requests::Workspace_Symbol::Result Server::onWorkspaceSymbol(
    lsp::requests::Workspace_Symbol::Params&& params
)
{
    std::vector<lsp::WorkspaceSymbol> symbols;
    const std::string& query = params.query;

    workspace_.forEachDocument([&](const std::string& docPath, OpenDocument& doc) {
        if (!doc.valid()) {
            return;
        }

        uint32_t schemaCount = bebop_result_schema_count(doc.result);
        for (uint32_t s = 0; s < schemaCount; ++s) {
            const bebop_schema_t* schema = bebop_result_schema_at(doc.result, s);
            const char* schemaPath = bebop_schema_path(schema);

            if (!schemaPath || docPath != schemaPath) {
                continue;
            }

            uint32_t defCount = bebop_schema_definition_count(schema);
            for (uint32_t d = 0; d < defCount; ++d) {
                const bebop_def_t* def = bebop_schema_definition_at(schema, d);
                const char* name = bebop_def_name(def);
                if (!name) {
                    continue;
                }

                // Fuzzy match: check if query characters appear in order
                if (!query.empty()) {
                    size_t qi = 0;
                    for (const char* p = name; *p && qi < query.size(); ++p) {
                        if (std::tolower(*p) == std::tolower(query[qi])) {
                            ++qi;
                        }
                    }
                    if (qi != query.size()) {
                        continue;
                    }
                }

                lsp::WorkspaceSymbol sym;
                sym.name = name;
                sym.kind = defKindToSymbolKind(bebop_def_kind(def));
                sym.location = lsp::Location{
                    .uri = doc.uri,
                    .range = doc.spanToRange(bebop_def_span(def))
                };
                symbols.push_back(std::move(sym));
            }
        }
    });

    return symbols;
}

lsp::requests::TextDocument_PrepareRename::Result Server::onPrepareRename(
    lsp::requests::TextDocument_PrepareRename::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc || !doc->valid()) {
        return {};
    }

    uint32_t offset = doc->positionToOffset(params.position);
    bebop_location_t loc;
    if (!bebop_result_locate(doc->result, path.c_str(), offset, &loc)) {
        return {};
    }

    if (loc.kind != BEBOP_LOC_DEF) {
        return {};
    }

    lsp::PrepareRenameResult result = doc->spanToRange(bebop_def_name_span(loc.def));
    return result;
}

lsp::requests::TextDocument_Rename::Result Server::onRename(
    lsp::requests::TextDocument_Rename::Params&& params
)
{
    std::string path(params.textDocument.uri.path());
    auto* doc = workspace_.getDocument(path);
    if (!doc || !doc->valid()) {
        return {};
    }

    uint32_t offset = doc->positionToOffset(params.position);
    bebop_location_t loc;
    if (!bebop_result_locate(doc->result, path.c_str(), offset, &loc)) {
        return {};
    }

    const bebop_def_t* def = nullptr;

    switch (loc.kind) {
        case BEBOP_LOC_FIELD_TYPE: {
            const bebop_type_t* type = bebop_field_type(loc.field);
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                def = bebop_type_resolved(type);
            }
            break;
        }
        case BEBOP_LOC_TYPE_REF: {
            const bebop_type_t* type = loc.type;
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                def = bebop_type_resolved(type);
            }
            break;
        }
        case BEBOP_LOC_DECORATOR:
            def = bebop_decorator_resolved(loc.decorator);
            break;
        case BEBOP_LOC_MIXIN: {
            const bebop_type_t* type = loc.type;
            if (bebop_type_kind(type) == BEBOP_TYPE_DEFINED) {
                def = bebop_type_resolved(type);
            }
            break;
        }
        case BEBOP_LOC_DEF:
            def = loc.def;
            break;
        default:
            return {};
    }

    if (!def) {
        return {};
    }

    const char* targetFqn = bebop_def_fqn(def);
    if (!targetFqn) {
        return {};
    }

    std::map<std::string, std::vector<lsp::TextEdit>> docEdits;
    std::map<std::string, lsp::DocumentUri> docUris;
    std::set<std::pair<std::string, uint32_t>> seenEdits;

    workspace_.forEachDocument([&](const std::string&, OpenDocument& otherDoc) {
        if (!otherDoc.valid()) {
            return;
        }

        std::string uriStr(otherDoc.uri.path());
        docUris[uriStr] = otherDoc.uri;

        uint32_t defCount = bebop_result_definition_count(otherDoc.result);
        for (uint32_t d = 0; d < defCount; ++d) {
            const bebop_def_t* otherDef = bebop_result_definition_at(otherDoc.result, d);
            const char* otherFqn = bebop_def_fqn(otherDef);
            if (!otherFqn || strcmp(otherFqn, targetFqn) != 0) {
                continue;
            }

            bebop_span_t nameSpan = bebop_def_name_span(otherDef);
            if (nameSpan.len > 0) {
                auto key = std::make_pair(uriStr, nameSpan.off);
                if (seenEdits.insert(key).second) {
                    docEdits[uriStr].push_back({
                        .range = otherDoc.spanToRange(nameSpan),
                        .newText = params.newName
                    });
                }
            }

            uint32_t refCount = bebop_def_references_count(otherDef);
            for (uint32_t i = 0; i < refCount; ++i) {
                bebop_span_t span = bebop_def_reference_at(otherDef, i);

                auto key = std::make_pair(uriStr, span.off);
                if (!seenEdits.insert(key).second) {
                    continue;
                }

                // Preserve namespace prefix for FQN references (e.g., "base.Color")
                std::string refText = otherDoc.content.substr(span.off, span.len);
                size_t lastDot = refText.rfind('.');
                std::string newText = (lastDot != std::string::npos)
                    ? refText.substr(0, lastDot + 1) + params.newName
                    : params.newName;

                docEdits[uriStr].push_back({
                    .range = otherDoc.spanToRange(span),
                    .newText = newText
                });
            }
        }
    });

    lsp::Array<lsp::OneOf<lsp::TextDocumentEdit, lsp::CreateFile, lsp::RenameFile, lsp::DeleteFile>> documentChanges;

    for (auto& [uriStr, edits] : docEdits) {
        // Sort edits in reverse order to prevent position shifting
        std::sort(edits.begin(), edits.end(), [](const lsp::TextEdit& a, const lsp::TextEdit& b) {
            if (a.range.start.line != b.range.start.line) {
                return a.range.start.line > b.range.start.line;
            }
            return a.range.start.character > b.range.start.character;
        });

        lsp::TextDocumentEdit docEdit;
        docEdit.textDocument.uri = docUris[uriStr];
        docEdit.textDocument.version = nullptr;
        docEdit.edits = lsp::Array<lsp::OneOf<lsp::TextEdit, lsp::AnnotatedTextEdit>>{};
        for (auto& e : edits) {
            docEdit.edits.push_back(std::move(e));
        }

        documentChanges.push_back(std::move(docEdit));
    }

    lsp::WorkspaceEdit edit;
    edit.documentChanges = std::move(documentChanges);
    return edit;
}

} // namespace beboplsp
