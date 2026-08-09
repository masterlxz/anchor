import 'package:flutter/material.dart';

import '../models/asset.dart';
import '../models/portfolio_transaction.dart';
import '../services/portfolio_repository.dart';

class AddTransactionScreen extends StatefulWidget {
  const AddTransactionScreen({super.key});

  @override
  State<AddTransactionScreen> createState() => _AddTransactionScreenState();
}

class _AddTransactionScreenState extends State<AddTransactionScreen> {
  final _repository = PortfolioRepository();
  final _quantityController = TextEditingController();
  final _unitPriceController = TextEditingController();
  final _totalValueController = TextEditingController();

  List<Asset> _assets = [];
  Asset? _selectedAsset;
  TransactionType _type = TransactionType.compra;
  DateTime _date = DateTime.now();
  bool _loading = true;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _loadAssets();
    _quantityController.addListener(() => setState(() {}));
    _unitPriceController.addListener(() => setState(() {}));
  }

  Future<void> _loadAssets() async {
    final assets = await _repository.listAssets();
    setState(() {
      _assets = assets;
      _selectedAsset = assets.isNotEmpty ? assets.first : null;
      _loading = false;
    });
  }

  double get _quantity => double.tryParse(_quantityController.text.replaceAll(',', '.')) ?? 0;
  double get _unitPrice => double.tryParse(_unitPriceController.text.replaceAll(',', '.')) ?? 0;
  double get _computedTotal => _quantity * _unitPrice;
  double get _manualTotal =>
      double.tryParse(_totalValueController.text.replaceAll(',', '.')) ?? 0;

  Future<void> _pickDate() async {
    final picked = await showDatePicker(
      context: context,
      initialDate: _date,
      firstDate: DateTime(2000),
      lastDate: DateTime.now(),
    );
    if (picked != null) setState(() => _date = picked);
  }

  Future<void> _save() async {
    final needsAsset = _type.needsAsset;
    final asset = _selectedAsset;
    if (needsAsset && asset?.id == null) return;

    final needsAmount = _type.needsQuantity && _type.needsUnitPrice;
    final quantity = needsAmount ? _quantity : null;
    final unitPrice = needsAmount ? _unitPrice : null;
    final totalValue = needsAmount ? _computedTotal : _manualTotal;
    if (needsAmount && (quantity! <= 0 || unitPrice! <= 0)) return;
    if (!needsAmount && totalValue <= 0) return;

    setState(() => _saving = true);

    await _repository.insertTransaction(PortfolioTransaction(
      assetId: needsAsset ? asset!.id : null,
      type: _type,
      quantity: quantity,
      unitPrice: unitPrice,
      totalValue: totalValue,
      date: _date,
      createdAt: DateTime.now(),
    ));

    if (mounted) Navigator.of(context).pop(true);
  }

  @override
  void dispose() {
    _quantityController.dispose();
    _unitPriceController.dispose();
    _totalValueController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }

    final needsAsset = _type.needsAsset;
    final needsAmount = _type.needsQuantity && _type.needsUnitPrice;
    final blockedByMissingAsset = needsAsset && _assets.isEmpty;

    return Scaffold(
      appBar: AppBar(title: const Text('Lançar transação')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            DropdownButtonFormField<TransactionType>(
              initialValue: _type,
              decoration: const InputDecoration(labelText: 'Tipo'),
              items: TransactionType.values
                  .map((t) => DropdownMenuItem(value: t, child: Text(t.label)))
                  .toList(),
              onChanged: (value) {
                if (value != null) setState(() => _type = value);
              },
            ),
            const SizedBox(height: 16),
            if (blockedByMissingAsset)
              const Text('Cadastre um ativo primeiro, na aba Portfolio.')
            else ...[
              if (needsAsset) ...[
                DropdownButtonFormField<Asset>(
                  initialValue: _selectedAsset,
                  decoration: const InputDecoration(labelText: 'Ativo'),
                  items: _assets
                      .map((a) => DropdownMenuItem(value: a, child: Text('${a.ticker} — ${a.name}')))
                      .toList(),
                  onChanged: (value) => setState(() => _selectedAsset = value),
                ),
                const SizedBox(height: 16),
              ],
              if (needsAmount) ...[
                TextField(
                  controller: _quantityController,
                  keyboardType: const TextInputType.numberWithOptions(decimal: true),
                  decoration: const InputDecoration(labelText: 'Quantidade'),
                ),
                const SizedBox(height: 16),
                TextField(
                  controller: _unitPriceController,
                  keyboardType: const TextInputType.numberWithOptions(decimal: true),
                  decoration: const InputDecoration(labelText: 'Preço unitário'),
                ),
                const SizedBox(height: 16),
                Text('Total: ${_computedTotal.toStringAsFixed(2)}'),
              ] else
                TextField(
                  controller: _totalValueController,
                  keyboardType: const TextInputType.numberWithOptions(decimal: true),
                  decoration: const InputDecoration(labelText: 'Valor'),
                ),
              const SizedBox(height: 16),
              ListTile(
                contentPadding: EdgeInsets.zero,
                title: Text('Data: ${_date.day.toString().padLeft(2, '0')}/'
                    '${_date.month.toString().padLeft(2, '0')}/${_date.year}'),
                trailing: const Icon(Icons.calendar_today),
                onTap: _pickDate,
              ),
              const SizedBox(height: 24),
              FilledButton(
                onPressed: _saving ? null : _save,
                child: const Text('Salvar'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
