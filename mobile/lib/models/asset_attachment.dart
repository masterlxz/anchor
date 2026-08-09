/// Um anexo (escritura, ITBI, cap table, ...) de um ativo `imovel`/
/// `empresaNaoListada` — espelha `desktop/src-tauri/src/entity/
/// asset_attachments.rs`. `stored_relative_path` é relativo ao diretório de
/// documentos do app (ver `PortfolioRepository.addAssetAttachment`), mesmo
/// espírito do `app_data_dir()` do desktop.
class AssetAttachment {
  final int? id;
  final int assetId;
  final String originalFileName;
  final String storedRelativePath;
  final int fileSizeBytes;
  final String? contentType;
  final String? documentType;
  final DateTime createdAt;

  const AssetAttachment({
    this.id,
    required this.assetId,
    required this.originalFileName,
    required this.storedRelativePath,
    required this.fileSizeBytes,
    this.contentType,
    this.documentType,
    required this.createdAt,
  });

  Map<String, Object?> toMap() {
    return {
      'id': id,
      'asset_id': assetId,
      'original_file_name': originalFileName,
      'stored_relative_path': storedRelativePath,
      'file_size_bytes': fileSizeBytes,
      'content_type': contentType,
      'document_type': documentType,
      'created_at': createdAt.toIso8601String(),
    };
  }

  factory AssetAttachment.fromMap(Map<String, Object?> map) {
    return AssetAttachment(
      id: map['id'] as int,
      assetId: map['asset_id'] as int,
      originalFileName: map['original_file_name'] as String,
      storedRelativePath: map['stored_relative_path'] as String,
      fileSizeBytes: map['file_size_bytes'] as int,
      contentType: map['content_type'] as String?,
      documentType: map['document_type'] as String?,
      createdAt: DateTime.parse(map['created_at'] as String),
    );
  }
}
