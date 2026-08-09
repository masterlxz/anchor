/// Uma entrada do histórico de avaliação manual de um ativo `imovel`/
/// `empresaNaoListada` — espelha `desktop/src-tauri/src/entity/
/// asset_valuations.rs`. `origin` sempre `"manual"` por ora (reajuste
/// automático por % ainda não decidido, mesma pendência do desktop) —
/// fixado por `PortfolioRepository.insertAssetValuation`, nunca digitado
/// pela tela.
class AssetValuation {
  final int? id;
  final int assetId;
  final DateTime valuationDate;
  final double value;
  final String origin;
  final String? notes;
  final DateTime createdAt;

  const AssetValuation({
    this.id,
    required this.assetId,
    required this.valuationDate,
    required this.value,
    required this.origin,
    this.notes,
    required this.createdAt,
  });

  Map<String, Object?> toMap() {
    return {
      'id': id,
      'asset_id': assetId,
      'valuation_date': valuationDate.toIso8601String(),
      'value': value,
      'origin': origin,
      'notes': notes,
      'created_at': createdAt.toIso8601String(),
    };
  }

  factory AssetValuation.fromMap(Map<String, Object?> map) {
    return AssetValuation(
      id: map['id'] as int,
      assetId: map['asset_id'] as int,
      valuationDate: DateTime.parse(map['valuation_date'] as String),
      value: map['value'] as double,
      origin: map['origin'] as String,
      notes: map['notes'] as String?,
      createdAt: DateTime.parse(map['created_at'] as String),
    );
  }
}
