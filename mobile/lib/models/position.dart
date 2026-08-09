import 'asset.dart';

/// Quantidade de um ativo numa custódia específica — `custodiaId == null`
/// significa "sem custódia definida". Sem rótulo embutido (diferente do
/// `CustodiaBreakdown` do desktop) — resolvido na tela a partir da lista de
/// custódias já carregada, mantendo `computePositions` livre de I/O.
class CustodiaBreakdown {
  final int? custodiaId;
  final double quantity;

  const CustodiaBreakdown({required this.custodiaId, required this.quantity});
}

/// Calculado a partir do histórico de transações, nunca persistido —
/// mesmo espírito de `get_portfolio_positions` no desktop
/// (`desktop/src-tauri/src/commands/transaction.rs`).
class Position {
  final Asset asset;
  final double netQuantity;

  /// `null` quando o ativo nunca teve uma compra registrada.
  final double? averageBuyPrice;
  final List<CustodiaBreakdown> byCustodia;

  const Position({
    required this.asset,
    required this.netQuantity,
    required this.averageBuyPrice,
    this.byCustodia = const [],
  });
}
