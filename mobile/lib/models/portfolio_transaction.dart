enum TransactionType {
  compra('compra', 'Compra'),
  venda('venda', 'Venda');

  final String value;
  final String label;

  const TransactionType(this.value, this.label);

  static TransactionType fromValue(String value) {
    return TransactionType.values.firstWhere((t) => t.value == value);
  }
}

class PortfolioTransaction {
  final int? id;
  final int assetId;
  final TransactionType type;
  final double quantity;
  final double unitPrice;
  final double totalValue;
  final DateTime date;
  final String? notes;
  final DateTime createdAt;

  const PortfolioTransaction({
    this.id,
    required this.assetId,
    required this.type,
    required this.quantity,
    required this.unitPrice,
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
      assetId: map['asset_id'] as int,
      type: TransactionType.fromValue(map['transaction_type'] as String),
      quantity: map['quantity'] as double,
      unitPrice: map['unit_price'] as double,
      totalValue: map['total_value'] as double,
      date: DateTime.parse(map['transaction_date'] as String),
      notes: map['notes'] as String?,
      createdAt: DateTime.parse(map['created_at'] as String),
    );
  }
}
