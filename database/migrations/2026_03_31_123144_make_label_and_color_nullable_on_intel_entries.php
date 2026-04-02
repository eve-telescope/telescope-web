<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::table('intel_entries', function (Blueprint $table) {
            $table->string('label')->nullable()->change();
            $table->string('color')->nullable()->change();
        });
    }

    public function down(): void
    {
        Schema::table('intel_entries', function (Blueprint $table) {
            $table->string('label')->nullable(false)->change();
            $table->string('color')->nullable(false)->change();
        });
    }
};
