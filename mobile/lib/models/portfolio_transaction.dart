/// 5 dos 6 tipos do desktop — `transferencia` fica de fora (depende de
/// custódia, que o mobile ainda não tem, ver `project/PHASE.md` Fase 10
/// item 9). Valores string iguais ao desktop
/// (`desktop/src/portfolio/types.ts`).
enum TransactionType {
  compra('compra', 'Compra'),
  venda('venda', 'Venda'),
  aporte('aporte', 'Aporte'),
  retirada('retirada', 'Retirada'),
  provento('provento', 'Provento');

  final String value;
  final String label;

  const TransactionType(this.value, this.label);

  static TransactionType fromValue(String value) {
    return TransactionType.values.firstWhere((t) => t.value == value);
  }
}

/// Quais campos cada tipo exige — espelha `needsAsset`/`needsQuantity`/
/// `needsUnitPrice` de `desktop/src/portfolio/TransactionSection.tsx`
/// (sem a parte de `transferencia`). `aporte`/`retirada` são fluxo de caixa
/// puro (sem ativo); `provento` tem ativo (qual pagou) mas não quantidade/
/// preço unitário, só um valor total digitado direto.
extension TransactionTypeMeta on TransactionType {
  bool get needsAsset => this != TransactionType.aporte && this != TransactionType.retirada;

  bool get needsQuantity => this == TransactionType.compra || this == TransactionType.venda;

  bool get needsUnitPrice => this == TransactionType.compra || this == TransactionType.venda;
}

class PortfolioTransaction {
  final int? id;

  /// `null` pra `aporte`/`retirada` (fluxo de caixa puro, sem ativo).
  final int? assetId;
  final TransactionType type;

  /// `null` pra tipos que não usam quantidade (ver `TransactionTypeMeta`).
  final double? quantity;

  /// `null` pra tipos que não usam preço unitário (ver `TransactionTypeMeta`).
  final double? unitPrice;
  final double totalValue;
  final DateTime date;
  final String? notes;
  final DateTime createdAt;

  const PortfolioTransaction({
    this.id,
    this.assetId,
    required this.type,
    this.quantity,
    this.unitPrice,
    required this.totalValue,
    required this.date,
    this.notes,
    required this.createdAt,
  });

  Map<String, Object?> toMap() {
    return {
      'id': id,
      'asset_id': assetId,
      'transaction_type': type.value,
      'quantity': quantity,
      'unit_price': unitPrice,
      'total_value': totalValue,
      'transaction_date': date.toIso8601String(),
      'notes': notes,
      'created_at': createdAt.toIso8601String(),
    };
  }

  factory PortfolioTransaction.fromMap(Map<String, Object?> map) {
    return PortfolioTransaction(
      id: map['id'] as int,
      assetId: map['asset_id'] as int?,
      type: TransactionType.fromValue(map['transaction_type'] as String),
      quantity: map['quantity'] as double?,
      unitPrice: map['unit_price'] as double?,
      totalValue: map['total_value'] as double,
      date: DateTime.parse(map['transaction_date'] as String),
      notes: map['notes'] as String?,
      createdAt: DateTime.parse(map['created_at'] as String),
    );
  }
}
