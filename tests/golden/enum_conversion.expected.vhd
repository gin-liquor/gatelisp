library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_enum_conversion is
  port (
    gl_p0_bits : in unsigned(4 downto 0);
    gl_p1_opcode : out unsigned(4 downto 0);
    gl_p2_debug : out unsigned(4 downto 0);
    gl_p3_low : out std_logic
  );
end entity gl_m0_enum_conversion;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_enum_conversion is
  signal gl_s1_opcode : unsigned(4 downto 0);
  signal gl_s2_debug : unsigned(4 downto 0);
  signal gl_s3_low : std_logic;
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  function gl_bit_at(value : unsigned; index : natural) return std_logic is
  begin
    return value(index);
  end function gl_bit_at;
begin
  gl_comb_0 : process(all)
  begin
    gl_s1_opcode <= gl_p0_bits;
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s2_debug <= gl_s1_opcode;
  end process gl_comb_1;
  gl_comb_2 : process(all)
  begin
    gl_s3_low <= gl_bit_at(gl_s1_opcode, 0);
  end process gl_comb_2;
  gl_p1_opcode <= gl_s1_opcode;
  gl_p2_debug <= gl_s2_debug;
  gl_p3_low <= gl_s3_low;
end architecture rtl;
