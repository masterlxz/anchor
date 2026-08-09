/// 4 classes resolvidas pelo mesmo endpoint Yahoo `.SA` já usado por
/// `YahooQuoteService` (confirmado no desktop, Sessões 41/63: FII/ETF
/// BR/BDR devolvem o mesmo formato de `meta` que uma ação), mais 3 classes
/// com fonte própria — `acaoInternacional` (Yahoo sem `.SA`), `metal`
/// (Yahoo sem `.SA`, ticker de contrato futuro por trás) e `cripto`
/// (CoinGecko, por `coin_id` em vez de ticker) — ver `QuoteDispatcher`.
/// Valores string iguais ao desktop (`desktop/src/portfolio/types.ts`).
enum AssetClass {
  acaoBr('acao_br', 'Ação BR'),
  fii('fii', 'FII'),
  etfBr('etf_br', 'ETF BR'),
  bdr('bdr', 'BDR'),
  acaoInternacional('acao_internacional', 'Ação internacional'),
  cripto('cripto', 'Cripto'),
  metal('metal', 'Metal');

  final String value;
  final String label;

  const AssetClass(this.value, this.label);

  static AssetClass fromValue(String value) {
    return AssetClass.values.firstWhere((c) => c.value == value);
  }
}

/// Ticker de exemplo e moeda default por classe — evita repetir o mesmo
/// switch em cada tela que precisa de um placeholder/moeda inicial.
extension AssetClassMeta on AssetClass {
  String get tickerHint {
    switch (this) {
      case AssetClass.acaoBr:
        return 'PETR4';
      case AssetClass.fii:
        return 'HGLG11';
      case AssetClass.etfBr:
        return 'BOVA11';
      case AssetClass.bdr:
        return 'AAPL34';
      case AssetClass.acaoInternacional:
        return 'AAPL';
      case AssetClass.cripto:
        return 'ETH';
      case AssetClass.metal:
        return 'XAU';
    }
  }

  String get defaultCurrency {
    switch (this) {
      case AssetClass.acaoBr:
      case AssetClass.fii:
      case AssetClass.etfBr:
      case AssetClass.bdr:
        return 'BRL';
      case AssetClass.acaoInternacional:
      case AssetClass.cripto:
      case AssetClass.metal:
        return 'USD';
    }
  }
}

class Asset {
  final int? id;
  final String ticker;
  final String name;
  final AssetClass assetClass;
  final String currency;
  final DateTime createdAt;

  /// `coin_id` do CoinGecko (ex: `ethereum`), resolvido e salvo no cadastro
  /// de um ativo `cripto` — evita rebuscar por texto a cada refresh do
  /// Portfolio. `null` pras outras 6 classes.
  final String? externalId;

  const Asset({
    this.id,
    required this.ticker,
    required this.name,
    required this.assetClass,
    required this.currency,
    required this.createdAt,
    this.externalId,
  });

  Map<String, Object?> toMap() {
    return {
      'id': id,
      'ticker': ticker,
      'name': name,
      'asset_class': assetClass.value,
      'currency': currency,
      'created_at': createdAt.toIso8601String(),
      'external_id': externalId,
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
      externalId: map['external_id'] as String?,
    );
  }
}
