/// As 4 classes desta fatia — todas resolvidas pelo mesmo endpoint Yahoo
/// `.SA` já usado por `YahooQuoteService` (confirmado no desktop, Sessões
/// 41/63: FII/ETF BR/BDR devolvem o mesmo formato de `meta` que uma ação).
enum AssetClass {
  acaoBr('acao_br', 'Ação BR'),
  fii('fii', 'FII'),
  etfBr('etf_br', 'ETF BR'),
  bdr('bdr', 'BDR');

  final String value;
  final String label;

  const AssetClass(this.value, this.label);

  static AssetClass fromValue(String value) {
    return AssetClass.values.firstWhere((c) => c.value == value);
  }
}

class Asset {
  final int? id;
  final String ticker;
  final String name;
  final AssetClass assetClass;
  final String currency;
  final DateTime createdAt;

  const Asset({
    this.id,
    required this.ticker,
    required this.name,
    required this.assetClass,
    required this.currency,
    required this.createdAt,
  });

  Map<String, Object?> toMap() {
    return {
      'id': id,
      'ticker': ticker,
      'name': name,
      'asset_class': assetClass.value,
      'currency': currency,
      'created_at': createdAt.toIso8601String(),
    };
  }

  factory Asset.fromMap(Map<String, Object?> map) {
    return Asset(
      id: map['id'] as int,
      ticker: map['ticker'] as String,
      name: map['name'] as String,
      assetClass: AssetClass.fromValue(map['asset_class'] as String),
      currency: map['currency'] as String,
      createdAt: DateTime.parse(map['created_at'] as String),
    );
  }
}
