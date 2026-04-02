<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('network_scans', function (Blueprint $table) {
            $table->id();
            $table->foreignId('intel_network_id')->constrained('intel_networks')->cascadeOnDelete();
            $table->foreignId('user_id')->constrained('users')->cascadeOnDelete();
            $table->string('scan_type');
            $table->longText('raw_text');
            $table->string('solar_system')->nullable();
            $table->timestamps();

            $table->index(['intel_network_id', 'created_at']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('network_scans');
    }
};
