import 'package:flutter/material.dart';

import '../models/asset.dart';
import '../models/portfolio_transaction.dart';
import '../services/portfolio_repository.dart';

/// Somente leitura — `aporte`/`retirada`/`provento` não aparecem em nenhuma
/// posição calculada (ver `PortfolioRepository.computePositions`), então
/// sem esta tela ficariam gravados no banco sem lugar nenhum pra aparecer.
class TransactionHistoryScreen extends StatefulWidget {
  const TransactionHistoryScreen({super.key});

  @override
  State<TransactionHistoryScreen> createState() => _TransactionHistoryScreenState();
}

class _TransactionHistoryScreenState extends State<TransactionHistoryScreen> {
  final _repository = PortfolioRepository();

  bool _loading = true;
  List<PortfolioTransaction> _transactions = [];
  Map<int, Asset> _assetsById = {};

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() => _loading = true);

    final transactions = await _repository.listAllTransactions();
    final assets = await _repository.listAssets();

    if (mounted) {
      setState(() {
        _transactions = transactions;
        _assetsById = {for (final a in assets) a.id!: a};
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Histórico')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _transactions.isEmpty
              ? const Center(child: Text('Nenhuma transação lançada ainda.'))
              : RefreshIndicator(
                  onRefresh: _load,
                  child: ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: _transactions.length,
                    itemBuilder: (context, index) {
                      final tx = _transactions[index];
                      final ticker = tx.assetId != null ? _assetsById[tx.assetId]?.ticker : null;
                      return _TransactionTile(transaction: tx, ticker: ticker);
                    },
                  ),
                ),
    );
  }
}

class _TransactionTile extends StatelessWidget {
  final PortfolioTransaction transaction;
  final String? ticker;

  const _TransactionTile({required this.transaction, required this.ticker});

  @override
  Widget build(BuildContext context) {
    final date = transaction.date;
    final dateLabel = '${date.day.toString().padLeft(2, '0')}/'
        '${date.month.toString().padLeft(2, '0')}/${date.year}';
    final quantity = transaction.quantity;

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ListTile(
        title: Text('${transaction.type.label} — ${ticker ?? '—'}'),
        subtitle: Text(
          '$dateLabel${quantity != null ? ' · ${quantity.toStringAsFixed(quantity == quantity.roundToDouble() ? 0 : 2)} un.' : ''}',
        ),
        trailing: Text(transaction.totalValue.toStringAsFixed(2)),
      ),
    );
  }
}
